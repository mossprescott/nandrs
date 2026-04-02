//! Alternate Hack CPU implementation, attempting to dispatch 2 Hack instructions per cycle
//! (sometimes).
//!
//! Observation: a common pattern in Hack programs is to load a value/address using a sequence like
//! "@1234; D=A" or "@R5; D=M". In any such sequence, the first instruction only needs the
//! instruction word and access to the A register, while the second instruction might access the
//! memory, branch, etc.
//!
//! Statistically, about 30% of all instructions in ROM are "A"-instructions (see Pong.asm).
//! Presumably they make up something like 30% of instructions *executed* as well. This architecture
//! will make all those instructions consume 0 cycles.
//!
//! What if the CPU could handle *both* a load to A *and* the ensuing instruction in a single cycle?
//! That would be a fun way to spend money on hardware, while maintaining compatibitly with the vast
//! and valuable library of Hack software.
//!
//! In actual implementation terms, it's simpler to flip that idea around: if the instruction
//! *after* the current instruction is "@...", then once we know the current instruction isn't going
//! to branch, we can fold the update of register A for the *following* instruction into the same
//! cycle. There's never a conflict, because whatever value the current instruction might have
//! written into A was going to be overwritten anyway, and "@-" instructions have *no* other
//! effects.
//!
//! This means for each such A-instruction, there will never be a cycle when PC points to that
//! particular instruction. Which probably won't cause confusion; we compare PC with the known
//! (labeled) addresses to keep track of progress, but a useful label can never be skipped
//! instruction:
//! - after a JMP, the target instruction is always dispatched alone, even if it's "@..."
//! - interesting labels are always jump targets: entry points, mainly
//!
//! To execute 2 instructions, we need to feed 2 instructions into the CPU on each cycle. Since we
//! don't have a dual-ported or double-clocked ROM in this project, we'll just fake it by wiring up
//! a second ROM which we'll load with the same binary.
use assignments::project_01::{And, And16, Buffer, Mux, Mux16, Nand, Not, Not16, Or};
use assignments::project_02::{ALU, Add16, FullAdder, HalfAdder, Inc16, Nand16Way, Neg16, Zero16};
use assignments::project_03::PC;
use assignments::project_05::{Decode, Project05Component};
use frunk::coproduct::CoprodInjector;
use frunk::{Coprod, hlist};
use simulator::component::{
    Computational, Computational16, MemorySystem16, ROM16, Register16, WiredRegister,
};
use simulator::declare::{BusRef, Interface};
use simulator::nat::N16;
use simulator::simulate::{BusResident, ChipState, ROMHandle};
use simulator::{
    self, Chip, Component, Flat, IC, Input1, Input16, Output, Output16, Reflect, expand_t, fixed,
    flatten_g,
};

/// CPU which (potentially) decodes and executes a pair of instructions in each cycle.
#[derive(Clone, Reflect, Chip)]
pub struct CPU {
    /// Return to a known state (i.e. jump to address 0)
    pub reset: Input1,

    /// Address of the next instructions to load
    pub pc0: Output16,
    pub pc1: Output16,

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

    fn expand(&self) -> Option<IC<Self::Target>> {
        Some(
            self.expand_t::<DoubleComponentT, _, _, _, _, _, _, _, _, _>()
                .map(Into::into),
        )
    }
}

impl CPU {
    expand_t!([Decode, Not, Or, Mux16, ALU, Buffer, And, Register16, DoublePC], |this| {
        // TODO: when the chip is powered on, DoublePC is in an invalid state (both out0 and out1 are 0).
        // A clever implementation here would detect that and assert "pc.reset" for one cycle automatically.

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
        not_jmp:  Not { a: do_jmp.out.into(), out: Output::new() },

        // Skip the following A-instr when not jumping:
        do_skip: And { a: decode1_is_a.out.into(), b: not_jmp.out.into(), out: Output::new() },

        // === A register data mux: AFTER ALU ===
        // sel=is_a → a1=instr (A-instr), a0=ALU output (C-instr with dest=A)
        a_data: Mux16 {
            a0:  this.mem_data_out.into(),
            a1:  this.instr0,
            sel: decode.is_a.into(),
            out: Output16::new(),
        },

        // Substitute the value of the *following* A-instr when we are able to "skip" that cycle:
        a_data_skip: Mux16 {
            a0: a_data.out.into(),
            a1: this.instr1,
            sel: do_skip.out.into(),
            out: Output16::new(),
        },

        // === A register: when skipping, load instr1 into A instead ===
        load_a_skip: Or { a: load_a.out.into(), b: do_skip.out.into(), out: Output::new() },

        // === next_addr: if A is being written this cycle (including via skip), expose the
        // new A value as the address for the memory system (so RAM latches the right read
        // address); otherwise expose the current A.out. ===
        next_addr: Mux16 {
            a0:  reg_a_out.into(),
            a1:  a_data_skip.out.into(),
            sel: load_a_skip.out.into(),
            out: this.mem_addr,
        },
        reg_a: Register16 { data_in: a_data_skip.out.into(), write: load_a_skip.out.into(), data_out: reg_a_out },

        // === D register (write_d already gated with is_c in Decode) ===
        reg_d: Register16 { data_in: this.mem_data_out.into(), write: decode.write_d.into(), data_out: reg_d_out },

        pc: DoublePC {
            reset: this.reset,
            addr:  reg_a_out.into(),
            load:  do_jmp.out.into(),
            skip:  do_skip.out.into(),
            out0:  this.pc0,
            out1:  this.pc1,
        },
    });
}

#[derive(Clone, Reflect, Chip)]
pub struct Computer {
    /// A way to force the CPU to return to a known state (i.e. jump to address 0)
    pub reset: Input1,

    /// Useful for debugging, but also acts as a root for traversing the graph
    pub pc: Output16,
}

impl Component for Computer {
    type Target = DoubleComponent;

    fn expand(&self) -> Option<IC<Self::Target>> {
        Some(self.expand_t::<DoubleComponentT, _, _, _>().map(Into::into))
    }
}

impl Computer {
    expand_t!([ROM16, CPU, MemorySystem16], |this| {
        mem_out: forward Output16::new(),
        pc1_out: forward Output16::new(),

        rom0: ROM16 {
            size: 32 * 1024,
            addr: this.pc.into(),
            out:  Output16::new(),
        },

        rom1: ROM16 {
            size: 32 * 1024,
            addr: pc1_out.into(),
            out:  Output16::new(),
        },

        cpu: CPU {
            reset:        this.reset,
            pc0:          this.pc,
            pc1:          pc1_out,
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
    });
}

/// PC with a "skip" input: when asserted, increment by 2 instead of 1.
/// load and reset take priority over skip (same precedence rules as project_03::PC).
/// in
#[derive(Clone, Reflect, Chip)]
pub struct DoublePC {
    pub reset: Input1,
    pub addr: Input16,
    pub load: Input1,
    /// When asserted (and not load/reset), increment by 2 instead of 1.
    pub skip: Input1,

    /// Address of the current instruction (latched)
    pub out0: Output16,
    /// Address of the next instruction; always equal to out0 + 1; also latched
    pub out1: Output16,
}

impl Component for DoublePC {
    type Target = DoubleComponent;

    fn expand(&self) -> Option<IC<Self::Target>> {
        Some(
            self.expand_t::<DoubleComponentT, _, _, _, _>()
                .map(Into::into),
        )
    }
}

impl DoublePC {
    expand_t!([Inc16, Inc2, Mux16, Register16], |this| {
        inc1: Inc16 { a: this.out0.into(), out: Output16::new() },
        inc2: Inc2 { a: this.out0.into(), out: Output16::new() },

        // skip=0 → inc by 1; skip=1 → inc by 2
        next0: Mux16 { a0: inc1.out.into(), a1: inc2.out.into(), sel: this.skip, out: Output16::new() },

        // load overrides inc/skip
        next1: Mux16 { a0: next0.out.into(), a1: this.addr, sel: this.load, out: Output16::new() },

        // reset overrides everything
        next2: Mux16 { a0: next1.out.into(), a1: fixed(0), sel: this.reset, out: Output16::new() },

        reg0: Register16 {
            data_in:  next2.out.into(),
            write:    fixed(1),
            data_out: this.out0,
        },

        inc3: Inc16 { a: next2.out.into(), out: Output16::new() },
        reg1: Register16 {
            data_in:  inc3.out.into(),
            write:    fixed(1),
            data_out: this.out1,
        },
    });
}

/// Add with the constant 2.
#[derive(Clone, Reflect, Chip)]
pub struct Inc2 {
    a: Input16,
    out: Output16,
}

impl Component for Inc2 {
    type Target = DoubleComponent;

    fn expand(&self) -> Option<IC<Self::Target>> {
        Some(self.expand_t::<DoubleComponentT, _, _, _>().map(Into::into))
    }
}

impl Inc2 {
    expand_t!([Buffer, Not, FullAdder], |this| {
        // the low bit is unaffected:
        low: Buffer { a: this.a.bit(0).into(), out: this.out.bit(0) },

        // the 2's place is always flipped:
        not1: Not { a: this.a.bit(1).into(), out: this.out.bit(1) },

        _carry_out: (2..16).fold(this.a.bit(1), |carry, i| {
            add: FullAdder {
                a: this.a.bit(i),
                b: fixed(0),
                c: carry,
                sum: this.out.bit(i),
                carry: Output::new(),
            },
            add.carry.into()
        }),
    });
}

pub type DoubleComponentT = Coprod!(
    Nand,
    Buffer,
    Not,
    And,
    Or,
    Mux,
    Mux16,
    Not16,
    And16,
    HalfAdder,
    FullAdder,
    Inc16,
    Add16,
    Nand16Way,
    Zero16,
    Neg16,
    ALU,
    Register16,
    PC,
    ROM16,
    MemorySystem16,
    Decode,
    CPU,
    Computer,
    DoublePC,
    Inc2
);

/// Deprecated.
#[derive(Clone, Reflect)]
pub enum DoubleComponent {
    Project05(Project05Component),
    CPU(CPU),
    Computer(Computer),
    DoublePC(DoublePC),
    Inc2(Inc2),
}

impl From<Project05Component> for DoubleComponent {
    fn from(c: Project05Component) -> Self {
        DoubleComponent::Project05(c)
    }
}

impl From<CPU> for DoubleComponent {
    fn from(c: CPU) -> Self {
        DoubleComponent::CPU(c)
    }
}

impl From<Computer> for DoubleComponent {
    fn from(c: Computer) -> Self {
        DoubleComponent::Computer(c)
    }
}

impl From<DoublePC> for DoubleComponent {
    fn from(c: DoublePC) -> Self {
        DoubleComponent::DoublePC(c)
    }
}

impl From<Inc2> for DoubleComponent {
    fn from(c: Inc2) -> Self {
        DoubleComponent::Inc2(c)
    }
}

impl Component for DoubleComponent {
    type Target = DoubleComponent;

    fn expand(&self) -> Option<IC<Self::Target>> {
        match self {
            DoubleComponent::Project05(c) => c
                .expand()
                .map(|ic| ic.map(|p| DoubleComponent::Project05(p))),
            DoubleComponent::CPU(c) => c.expand(),
            DoubleComponent::Computer(c) => c.expand(),
            DoubleComponent::DoublePC(c) => c.expand(),
            DoubleComponent::Inc2(c) => c.expand(),
        }
    }
}

// TEMP
impl From<DoubleComponentT> for DoubleComponent {
    fn from(comp: DoubleComponentT) -> Self {
        use assignments::project_01::Project01Component;
        use assignments::project_02::Project02Component;
        use assignments::project_03::Project03Component;
        use assignments::project_05::Project05Component;
        comp.fold(hlist![
            |c: Nand| DoubleComponent::Project05(Project05Component::Project03(
                Project03Component::Project02(Project02Component::Project01(
                    Project01Component::Nand(c),
                ))
            )),
            |c: Buffer| DoubleComponent::Project05(Project05Component::Project03(
                Project03Component::Project02(Project02Component::Project01(
                    Project01Component::Buffer(c),
                ))
            )),
            |c: Not| DoubleComponent::Project05(Project05Component::Project03(
                Project03Component::Project02(Project02Component::Project01(
                    Project01Component::Not(c),
                ))
            )),
            |c: And| DoubleComponent::Project05(Project05Component::Project03(
                Project03Component::Project02(Project02Component::Project01(
                    Project01Component::And(c),
                ))
            )),
            |c: Or| DoubleComponent::Project05(Project05Component::Project03(
                Project03Component::Project02(Project02Component::Project01(
                    Project01Component::Or(c),
                ))
            )),
            |c: Mux| DoubleComponent::Project05(Project05Component::Project03(
                Project03Component::Project02(Project02Component::Project01(
                    Project01Component::Mux(c),
                ))
            )),
            |c: Mux16| DoubleComponent::Project05(Project05Component::Project03(
                Project03Component::Project02(Project02Component::Project01(
                    Project01Component::Mux16(c),
                ))
            )),
            |c: Not16| DoubleComponent::Project05(Project05Component::Project03(
                Project03Component::Project02(Project02Component::Project01(
                    Project01Component::Not16(c),
                ))
            )),
            |c: And16| DoubleComponent::Project05(Project05Component::Project03(
                Project03Component::Project02(Project02Component::Project01(
                    Project01Component::And16(c),
                ))
            )),
            |c: HalfAdder| DoubleComponent::Project05(Project05Component::Project03(
                Project03Component::Project02(Project02Component::HalfAdder(c))
            )),
            |c: FullAdder| DoubleComponent::Project05(Project05Component::Project03(
                Project03Component::Project02(Project02Component::FullAdder(c))
            )),
            |c: Inc16| DoubleComponent::Project05(Project05Component::Project03(
                Project03Component::Project02(Project02Component::Inc16(c))
            )),
            |c: Add16| DoubleComponent::Project05(Project05Component::Project03(
                Project03Component::Project02(Project02Component::Add16(c))
            )),
            |c: Nand16Way| DoubleComponent::Project05(Project05Component::Project03(
                Project03Component::Project02(Project02Component::Nand16Way(c))
            )),
            |c: Zero16| DoubleComponent::Project05(Project05Component::Project03(
                Project03Component::Project02(Project02Component::Zero16(c))
            )),
            |c: Neg16| DoubleComponent::Project05(Project05Component::Project03(
                Project03Component::Project02(Project02Component::Neg16(c))
            )),
            |c: ALU| DoubleComponent::Project05(Project05Component::Project03(
                Project03Component::Project02(Project02Component::ALU(c))
            )),
            |c: Register16| DoubleComponent::Project05(Project05Component::Project03(
                Project03Component::Register(c)
            )),
            |c: PC| DoubleComponent::Project05(Project05Component::Project03(
                Project03Component::PC(c)
            )),
            |c: ROM16| DoubleComponent::Project05(Project05Component::ROM(c)),
            |c: MemorySystem16| DoubleComponent::Project05(Project05Component::MemorySystem(c)),
            |c: Decode| DoubleComponent::Project05(Project05Component::Decode(c)),
            DoubleComponent::CPU,
            DoubleComponent::Computer,
            DoubleComponent::DoublePC,
            DoubleComponent::Inc2,
        ])
    }
}

/// Find the two ROMs (rom0 at pc, rom1 at pc+1) in the chip state.
pub fn find_roms(state: &ChipState<N16, N16>) -> (ROMHandle<N16, N16>, ROMHandle<N16, N16>) {
    let roms: Vec<_> = state
        .bus_residents()
        .iter()
        .filter_map(|r| {
            if let BusResident::ROM(h) = r {
                Some(h.clone())
            } else {
                None
            }
        })
        .collect();
    assert_eq!(roms.len(), 2, "expected 2 ROMs, found {}", roms.len());
    (roms[0].clone(), roms[1].clone())
}

/// Recursively expand until only Nands, Registers, RAMs, ROMs, and MemorySystems are left.
pub fn flatten_t<C, Idx>(chip: C) -> IC<Computational16>
where
    C: Reflect,
    DoubleComponentT: CoprodInjector<C, Idx>,
{
    flatten_g::<C, DoubleComponentT, Idx, Computational16, _>(
        chip,
        "flat",
        hlist![
            |c: Nand| Flat::Done(vec![Computational::Nand(c)]),
            |c: Buffer| Flat::Done(vec![Computational::Buffer(c)]),
            |c: Not| Flat::Continue(c.expand_t()),
            |c: And| Flat::Continue(c.expand_t()),
            |c: Or| Flat::Continue(c.expand_t()),
            |c: Mux| Flat::Continue(c.expand_t()),
            |c: Mux16| Flat::Continue(c.expand_t()),
            |c: Not16| Flat::Continue(c.expand_t()),
            |c: And16| Flat::Continue(c.expand_t()),
            |c: HalfAdder| Flat::Continue(c.expand_t()),
            |c: FullAdder| Flat::Continue(c.expand_t()),
            |c: Inc16| Flat::Continue(c.expand_t()),
            |c: Add16| Flat::Continue(c.expand_t()),
            |c: Nand16Way| Flat::Continue(c.expand_t()),
            |c: Zero16| Flat::Continue(c.expand_t()),
            |c: Neg16| Flat::Continue(c.expand_t()),
            |c: ALU| Flat::Continue(c.expand_t()),
            |c: Register16| Flat::Done(vec![Computational::Register(WiredRegister::from(c))]),
            |c: PC| Flat::Continue(c.expand_t()),
            |c: ROM16| Flat::Done(vec![Computational::ROM(c)]),
            |c: MemorySystem16| Flat::Done(vec![Computational::MemorySystem(c)]),
            |c: Decode| Flat::Continue(c.expand_t()),
            |c: CPU| Flat::Continue(c.expand_t()),
            |c: Computer| Flat::Continue(c.expand_t()),
            |c: DoublePC| Flat::Continue(c.expand_t()),
            |c: Inc2| Flat::Continue(c.expand_t()),
        ],
    )
}

/// Like `flatten_t`, but uses native Mux/Adder components for efficient simulation.
pub fn flatten_for_simulation<C, Idx>(
    chip: C,
) -> IC<simulator::component::native::Simulational<N16, N16>>
where
    C: Reflect,
    DoubleComponentT: CoprodInjector<C, Idx>,
{
    use simulator::component::native;
    flatten_g::<C, DoubleComponentT, Idx, native::Simulational<N16, N16>, _>(
        chip,
        "flat/sim",
        hlist![
            // Delegate all Project02 types to project_02::flatten_for_simulation:
            |c: Nand| Flat::Done(assignments::project_02::flatten_for_simulation(c).components),
            |c: Buffer| Flat::Done(assignments::project_02::flatten_for_simulation(c).components),
            |c: Not| Flat::Done(assignments::project_02::flatten_for_simulation(c).components),
            |c: And| Flat::Done(assignments::project_02::flatten_for_simulation(c).components),
            |c: Or| Flat::Done(assignments::project_02::flatten_for_simulation(c).components),
            |c: Mux| Flat::Done(assignments::project_02::flatten_for_simulation(c).components),
            |c: Mux16| Flat::Done(assignments::project_02::flatten_for_simulation(c).components),
            |c: Not16| Flat::Done(assignments::project_02::flatten_for_simulation(c).components),
            |c: And16| Flat::Done(assignments::project_02::flatten_for_simulation(c).components),
            |c: HalfAdder| Flat::Done(
                assignments::project_02::flatten_for_simulation(c).components
            ),
            |c: FullAdder| Flat::Done(
                assignments::project_02::flatten_for_simulation(c).components
            ),
            |c: Inc16| Flat::Done(assignments::project_02::flatten_for_simulation(c).components),
            |c: Add16| Flat::Done(assignments::project_02::flatten_for_simulation(c).components),
            |c: Nand16Way| Flat::Done(
                assignments::project_02::flatten_for_simulation(c).components
            ),
            |c: Zero16| Flat::Done(assignments::project_02::flatten_for_simulation(c).components),
            |c: Neg16| Flat::Done(assignments::project_02::flatten_for_simulation(c).components),
            |c: ALU| Flat::Done(assignments::project_02::flatten_for_simulation(c).components),
            // Project05/Double-specific types:
            |c: Register16| Flat::Done(vec![
                Computational::Register(WiredRegister::from(c)).into()
            ]),
            |c: PC| Flat::Continue(c.expand_t()),
            |c: ROM16| Flat::Done(vec![Computational::ROM(c).into()]),
            |c: MemorySystem16| Flat::Done(vec![Computational::MemorySystem(c).into()]),
            |c: Decode| Flat::Continue(c.expand_t()),
            |c: CPU| Flat::Continue(c.expand_t()),
            |c: Computer| Flat::Continue(c.expand_t()),
            |c: DoublePC| Flat::Continue(c.expand_t()),
            |c: Inc2| Flat::Continue(c.expand_t()),
        ],
    )
}

#[cfg(test)]
mod test {
    use assignments::project_05::memory_system;
    use assignments::tests::test_05;
    use simulator::Chip;
    use simulator::component::Computational;
    use simulator::print_graph;
    use simulator::simulate::simulate;

    use crate::computer::{Computer, find_roms, flatten_t};

    #[test]
    fn computer_max_behavior() {
        let chip = Computer::chip();

        // When it breaks, it's nice to see what it tried to do
        println!("{}", print_graph(&chip));

        let flat = flatten_t(chip);
        let state = simulate(&flat, memory_system());

        let (rom0, rom1) = find_roms(&state);

        let pgm = test_05::max_program();
        rom0.flash(pgm.clone());
        rom1.flash(pgm.clone());

        test_05::test_computer_max_behavior(state, pgm.len() as u64);
    }

    #[test]
    fn computer_optimal() {
        let components = flatten_t(Computer::chip()).components;
        let memsys = components
            .iter()
            .filter(|c| matches!(c, Computational::MemorySystem(_)))
            .count();
        let roms = components
            .iter()
            .filter(|c| matches!(c, Computational::ROM(_)))
            .count();
        let nands = components
            .iter()
            .filter(|c| matches!(c, Computational::Nand(_)))
            .count();
        let registers = components
            .iter()
            .filter(|c| matches!(c, Computational::Register(_)))
            .count();
        assert_eq!(memsys, 1);
        assert_eq!(roms, 2); // Compare to 1
        assert_eq!(nands, 1385); // Compare to 1126
        assert_eq!(registers, 4); // Compare to 3
    }
}
