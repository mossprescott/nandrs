/// Alternate Hack CPU implementation, attempting to dispatch 2 Hack instructions per cycle
/// (sometimes).
///
/// Observation: a common pattern in Hack programs is to load a value/address using a sequence like
/// "@1234; D=A" or "@R5; D=M". In any such sequence, the first instruction only needs the
/// instruction word and access to the A register, while the second instruction might access the
/// memory, branch, etc.
///
/// Statistically, about 30% of all instructions in ROM are "A"-instructions (see Pong.asm).
/// Presumably they make up something like 30% of instructions *executed* as well. This architecture
/// will make all those instructions consume 0 cycles.
///
/// What if the CPU could handle *both* a load to A *and* the ensuing instruction in a single cycle?
/// That would be a fun way to spend money on hardware, while maintaining compatibitly with the vast
/// and valuable library of Hack software.
///
/// In actual implementation terms, it's simpler to flip that idea around: if the instruction
/// *after* the current instruction is "@...", then once we know the current instruction isn't going
/// to branch, we can fold the update of register A for the *following* instruction into the same
/// cycle. There's never a conflict, because whatever value the current instruction might have
/// written into A was going to be overwritten anyway, and "@-" instructions have *no* other
/// effects.
///
/// This means for each such A-instruction, there will never be a cycle when PC points to that
/// particular instruction. Which probably won't cause confusion; we compare PC with the known
/// (labeled) addresses to keep track of progress, but a useful label can never be skipped
/// instruction:
/// - after a JMP, the target instruction is always dispatched alone, even if it's "@..."
/// - interesting labels are always jump targets: entry points, mainly
///
/// To execute 2 instructions, we need to feed 2 instructions into the CPU on each cycle. Since we
/// don't have a dual-ported or double-clocked ROM in this project, we'll just fake it by wiring up
/// a second ROM which we'll load with the same binary.

use assignments::project_01::{And, Or, Not};
use assignments::project_02::{ALU, Add16, Inc16};
use assignments::project_03::PC;
use assignments::project_05::{self, Decode, Project05Component};
use simulator::{self, AsConst, Component, IC, Input, Input16, Output, Output16, Reflect, Chip, expand};
use simulator::component::{Buffer, Computational16, Const, Mux16, MemorySystem16, Register16, ROM16};
use simulator::nat::N16;
use simulator::simulate::{ChipState, BusResident, ROMHandle};

/// CPU which (potentially) decodes and executes a pair of instructions in each cycle.
#[derive(Reflect, Chip)]
pub struct CPU {
    /// Return to a known state (i.e. jump to address 0)
    pub reset: Input,

    /// Address of the next instruction to load
    pub pc: Output16,

    /// The bits of the current instruction
    pub instr0: Input16,

    /// The bits of the instruction following the current instruction
    pub instr1: Input16,

    pub mem_data_out: Output16,
    pub mem_write: Output,

    pub mem_addr: Output16,

    pub mem_data_in: Input16,
}

impl Component for CPU {
    type Target = DoubleComponent;

    expand! { |this| {
        // Forward-declare register outputs:
        reg_a_out: forward Output16::new(),
        reg_d_out: forward Output16::new(),

        decode: Decode {
            instr: this.instr0.into(),
            is_c: Output::new(), is_a: Output::new(),
            read_m: Output::new(),
            zx: Output::new(), nx: Output::new(),
            zy: Output::new(), ny: Output::new(),
            f: Output::new(), no: Output::new(),
            write_a: Output::new(), write_m: Output::new(), write_d: Output::new(),
            jmp_lt: Output::new(), jmp_eq: Output::new(), jmp_gt: Output::new(),
        },

        // Minimal decode for the second instr:
        decode1_is_a: Not { a: this.instr1.bit(15).into(), out: Output::new() },

        // if:
        // - instr0 does not result in a jump (after ALU)
        // - decode1_is_a is true
        // then:
        // - all the usual handling of instr0
        // - incr PC by 2
        // - copy instr1 to A

        // === load_a = is_a OR write_a ===
        load_a: Or { a: decode.is_a.into(), b: decode.write_a.into(), out: Output::new() },

        // === ALU Y mux: sel=read_m → a0=A, a1=mem_in ===
        y_src: Mux16 {
            sel: decode.read_m.into(),
            a0:  reg_a_out.into(),
            a1:  this.mem_data_in,
            out: Output16::new(),
        },

        // === ALU: x=D, y=y_src, enabled only on C-instructions ===
        alu: ALU {
            x:   reg_d_out.into(),
            y:   y_src.out.into(),
            zx:  decode.zx.into(), nx: decode.nx.into(),
            zy:  decode.zy.into(), ny: decode.ny.into(),
            f:   decode.f.into(),  no: decode.no.into(),
            disable: decode.is_a.into(),
            out: this.mem_data_out,
            zr:  Output::new(),
            ng:  Output::new(),
        },

        // === A register data mux: AFTER ALU ===
        // sel=is_a → a1=instr (A-instr), a0=ALU output (C-instr with dest=A)
        a_data: Mux16 {
            sel: decode.is_a.into(),
            a0:  this.mem_data_out.into(),
            a1:  this.instr0,
            out: Output16::new(),
        },

        // === next_addr: if A is being written this cycle, expose the new A value as the
        // address for the memory system (so RAM latches the right read address); otherwise
        // expose the current A.out. Write address is always A.out (load_a=0 when write_m=1). ===
        next_addr: Mux16 {
            sel: load_a.out.into(),
            a0:  reg_a_out.into(),
            a1:  a_data.out.into(),
            out: this.mem_addr,
        },

        // === A register ===
        reg_a: Register16 { data_in: a_data.out.into(), write: load_a.out.into(), data_out: reg_a_out },

        // === D register (write_d already gated with is_c in Decode) ===
        reg_d: Register16 { data_in: this.mem_data_out.into(), write: decode.write_d.into(), data_out: reg_d_out },

        // === mem_write (write_m already gated with is_c in Decode) ===
        mem_write_buf: Buffer { a: decode.write_m.into(), out: this.mem_write },

        // === Jump logic ===
        not_ng:   Not { a: alu.ng.into(), out: Output::new() },
        not_zr:   Not { a: alu.zr.into(), out: Output::new() },
        is_pos:   And { a: not_ng.out.into(), b: not_zr.out.into(), out: Output::new() },
        // Jump signals already gated with is_c in Decode.
        jlt_and:  And { a: decode.jmp_lt.into(), b: alu.ng.into(), out: Output::new() },
        jeq_and:  And { a: decode.jmp_eq.into(), b: alu.zr.into(), out: Output::new() },
        jgt_and:  And { a: decode.jmp_gt.into(), b: is_pos.out.into(), out: Output::new() },
        j_lt_eq:  Or  { a: jlt_and.out.into(), b: jeq_and.out.into(), out: Output::new() },
        do_jmp:   Or  { a: j_lt_eq.out.into(), b: jgt_and.out.into(), out: Output::new() },

        // Next PC if we're dispatching a second instr
        skip_pc: Inc2 { a: this.pc.into(), out: Output16::new() },

        load_pc_addr: Mux16 { a0: skip_pc.out.into(), a1: reg_a_out.into(), sel: do_jmp.out.into(), out: Output16::new() },

        // Either we jump or we dispatch the following A-instr:
        load_pc: Or  { a: do_jmp.out.into(), b: decode1_is_a.out.into(), out: Output::new() },

        // === PC: inc always 1 ===
        const_one: Const { value: 1, out: Output::new() },
        // TODO: implement custom PC with a "skip" input?
        pc: PC {
            reset: this.reset.into(),
            addr:  load_pc_addr.out.into(),
            load:  load_pc.out.into(),
            inc:   const_one.out.bit(0).into(),
            out:   this.pc,
        },
    }}
}


#[derive(Reflect, Chip)]
pub struct Computer {
    /// A way to force the CPU to return to a known state (i.e. jump to address 0)
    pub reset: Input,

    /// Useful for debugging, but also acts as a root for traversing the graph
    pub pc: Output16,
}

impl Component for Computer {
    type Target = DoubleComponent;

    expand! { |this| {
        mem_out: forward Output16::new(),

        rom0: ROM16 {
            size: 32 * 1024,
            addr: this.pc.into(),
            out:  Output16::new(),
        },

        next_pc: Inc16 {
            a: this.pc.into(),
            out: Output16::new(),
        },
        rom1: ROM16 {
            size: 32 * 1024,
            addr: next_pc.out.into(),
            out:  Output16::new(),
        },

        cpu: CPU {
            reset:        this.reset,
            pc:           this.pc,
            instr0:       rom0.out.into(),
            instr1:       rom1.out.into(),
            mem_data_out: Output16::new(),
            mem_write:    Output::new(),
            mem_addr:     Output16::new(),
            mem_data_in:  mem_out.into(),
        },

        memory: MemorySystem16 {
            addr:     cpu.mem_addr.into(),
            write:    cpu.mem_write.into(),
            data_in:  cpu.mem_data_out.into(),
            data_out: mem_out,
        },
    }}
}

/// Add with the constant 2.
#[derive(Reflect, Chip)]
pub struct Inc2 {
    a: Input16,
    out: Output16,
}

impl Component for Inc2 {
    type Target = DoubleComponent;

    expand! { |this| {
        // TODO: construct from 14 half-adders; for minimal gate count
        // _: Buffer { a: this.a.bit(0), out: this.out.bit(0).into() },

        // Adding a constant value is just as efficient in simulation:
        two: Const { value: 2, out: Output16::new() },
        _add: Add16 { a: this.a, b: two.out.into(), out: this.out },
    }}
}


pub enum DoubleComponent {
    Project05(Project05Component),
    CPU(CPU),
    Computer(Computer),
    Inc2(Inc2),
}

impl<C: Into<Project05Component>> From<C> for DoubleComponent {
    fn from(c: C) -> Self {
        DoubleComponent::Project05(c.into())
    }
}
impl From<CPU>      for DoubleComponent { fn from(c: CPU)      -> Self { DoubleComponent::CPU(c)      } }
impl From<Computer> for DoubleComponent { fn from(c: Computer) -> Self { DoubleComponent::Computer(c) } }
impl From<Inc2>     for DoubleComponent { fn from(c: Inc2)     -> Self { DoubleComponent::Inc2(c) } }

impl Component for DoubleComponent {
    type Target = DoubleComponent;

    fn expand(&self) -> Option<IC<DoubleComponent>> {
        match self {
            DoubleComponent::Project05(c) => c.expand().map(|ic| IC { name: ic.name, intf: ic.intf, components: ic.components.into_iter().map(Into::into).collect() }),
            DoubleComponent::CPU(c)       => c.expand(),
            DoubleComponent::Computer(c)  => c.expand(),
            DoubleComponent::Inc2(c)      => c.expand(),
        }
    }
}

impl Reflect for DoubleComponent {
    fn reflect(&self) -> simulator::Interface {
        match self {
            DoubleComponent::Project05(c) => c.reflect(),
            DoubleComponent::CPU(c)       => c.reflect(),
            DoubleComponent::Computer(c)  => c.reflect(),
            DoubleComponent::Inc2(c)      => c.reflect(),
        }
    }
    fn name(&self) -> String {
        match self {
            DoubleComponent::Project05(c) => c.name(),
            DoubleComponent::CPU(c)       => c.name(),
            DoubleComponent::Computer(c)  => c.name(),
            DoubleComponent::Inc2(c)      => c.name(),
        }
    }
}

/// Find the two ROMs (rom0 at pc, rom1 at pc+1) in the chip state.
pub fn find_roms(state: &ChipState<N16, N16>) -> (ROMHandle<N16, N16>, ROMHandle<N16, N16>) {
    let roms: Vec<_> = state.bus_residents().iter()
        .filter_map(|r| if let BusResident::ROM(h) = r { Some(h.clone()) } else { None })
        .collect();
    assert_eq!(roms.len(), 2, "expected 2 ROMs, found {}", roms.len());
    (roms[0].clone(), roms[1].clone())
}

/// Recursively expand until only Nands, Registers, RAMs, ROMs, and MemorySystems are left.
pub fn flatten<C: Reflect + Into<DoubleComponent>>(chip: C) -> IC<Computational16> {
    fn go(comp: DoubleComponent) -> Vec<Computational16> {
        match comp.expand() {
            None => match comp {
                DoubleComponent::Project05(p) =>
                    project_05::flatten(p)
                        .components,
                _ => panic!("Did not reduce to primitive: {:?}", comp.name()),
            },
            Some(ic) => ic.components.into_iter().flat_map(go).collect(),
        }
    }
    IC {
        name: format!("{} (flat)", chip.name()),
        intf: chip.reflect(),
        components: go(chip.into()),
    }
}

impl AsConst for DoubleComponent {
    fn as_const(&self) -> Option<u64> {
        if let DoubleComponent::Project05(c) = self { c.as_const() } else { None }
    }
}

#[cfg(test)]
mod test {
    use assignments::project_05::memory_system;
    use assignments::tests::test_05;
    use simulator::Chip;
    use simulator::component::Computational;
    use simulator::print_graph;
    use simulator::simulate::simulate;

    use crate::computer::{Computer, find_roms, flatten};

    #[test]
    fn computer_max_behavior() {
        let chip = Computer::chip();

        // When it breaks, it's nice to see what it tried to do
        println!("{}", print_graph(&chip));

        let flat = flatten(chip);
        let state = simulate(&flat, memory_system());

        let (rom0, rom1) = find_roms(&state);

        let pgm = test_05::max_program();
        rom0.flash(pgm.clone());
        rom1.flash(pgm.clone());

        test_05::test_computer_max_behavior(state, pgm.len() as u64);
    }

    #[test]
    fn computer_optimal() {
        let components = flatten(Computer::chip()).components;
        let memsys = components.iter().filter(|c| matches!(c, Computational::MemorySystem(_))).count();
        let roms   = components.iter().filter(|c| matches!(c, Computational::ROM(_))).count();
        let nands  = components.iter().filter(|c| matches!(c, Computational::Nand(_))).count();
        let adders = components.iter().filter(|c| matches!(c, Computational::Adder(_))).count();
        let muxes  = components.iter().filter(|c| matches!(c, Computational::Mux(_))).count();
        assert_eq!(memsys,  1);
        assert_eq!(roms,    2);    // Compare to 1
        assert_eq!(nands, 171);    // Compare to 166
        assert_eq!(adders, 62);    // Compare to 31
        assert_eq!(muxes,  16);    // Compare to 15
    }
}