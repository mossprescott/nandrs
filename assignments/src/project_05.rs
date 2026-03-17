#![allow(unused_variables, dead_code, unused_imports)]

use simulator::{self, Component, IC, Input, Input16, Output, Output16, Reflect, AsConst, Chip};
use simulator::Reflect as _;
use simulator::Chip as _;
use simulator::component::{Buffer, Nand, Register16, RAM16, ROM16, MemorySystem16, Sequential, Computational, Computational16};
use simulator::nat::N16;
use simulator::simulate::{ChipState, BusResident, ROMHandle, RAMHandle, SerialHandle, MemoryMap, RegionMap, RAMMap, ROMMap, SerialMap};
use simulator::word::Word16;
use crate::project_01::{Project01Component, Const, Not, And, Or, Mux16};
use crate::project_02::{Project02Component, ALU};
use crate::project_03::{Project03Component, PC};

pub enum Project05Component {
    Project03(Project03Component),
    ROM(ROM16),
    MemorySystem(MemorySystem16),
    Decode(Decode),
    CPU(CPU),
    Computer(Computer),
}

impl From<Project03Component> for Project05Component { fn from(c: Project03Component) -> Self { Project05Component::Project03(c)    } }
impl From<ROM16>              for Project05Component { fn from(c: ROM16)              -> Self { Project05Component::ROM(c)           } }
impl From<MemorySystem16>     for Project05Component { fn from(c: MemorySystem16)     -> Self { Project05Component::MemorySystem(c)  } }
impl From<Decode>             for Project05Component { fn from(c: Decode)             -> Self { Project05Component::Decode(c)        } }
impl From<CPU>                for Project05Component { fn from(c: CPU)                -> Self { Project05Component::CPU(c)           } }
impl From<Computer>           for Project05Component { fn from(c: Computer)           -> Self { Project05Component::Computer(c)      } }

impl Component for Project05Component {
    type Target = Project05Component;

    fn expand(&self) -> Option<IC<Project05Component>> {
        match self {
            Project05Component::Project03(c)    => c.expand().map(|ic| IC { name: ic.name, intf: ic.intf, components: ic.components.into_iter().map(Into::into).collect() }),
            Project05Component::ROM(c)          => c.expand().map(|_| unreachable!()),
            Project05Component::MemorySystem(c) => c.expand().map(|_| unreachable!()),
            Project05Component::Decode(c)       => c.expand(),
            Project05Component::CPU(c)          => c.expand(),
            Project05Component::Computer(c)     => c.expand(),
        }
    }
}

impl Reflect for Project05Component {
    fn reflect(&self) -> simulator::Interface {
        match self {
            Project05Component::Project03(c)    => c.reflect(),
            Project05Component::ROM(c)          => c.reflect(),
            Project05Component::MemorySystem(c) => c.reflect(),
            Project05Component::Decode(c)       => c.reflect(),
            Project05Component::CPU(c)          => c.reflect(),
            Project05Component::Computer(c)     => c.reflect(),
        }
    }
    fn name(&self) -> String {
        match self {
            Project05Component::Project03(c)    => c.name(),
            Project05Component::ROM(c)          => c.name(),
            Project05Component::MemorySystem(c) => c.name(),
            Project05Component::Decode(c)       => c.name(),
            Project05Component::CPU(c)          => c.name(),
            Project05Component::Computer(c)     => c.name(),
        }
    }
}

impl AsConst for Project05Component {
    fn as_const(&self) -> Option<u64> {
        if let Project05Component::Project03(c) = self { c.as_const() } else { None }
    }
}

fn p01<C: Into<Project01Component>>(c: C) -> Project05Component {
    let p01: Project01Component = c.into();
    let p02: Project02Component = p01.into();
    let p03: Project03Component = p02.into();
    p03.into()
}
fn p02<C: Into<Project02Component>>(c: C) -> Project05Component {
    let p02: Project02Component = c.into();
    let p03: Project03Component = p02.into();
    p03.into()
}
fn p03<C: Into<Project03Component>>(c: C) -> Project05Component {
    let p03: Project03Component = c.into();
    p03.into()
}

/// Recursively expand until only Nands, Registers, RAMs, and ROMs are left.
pub fn flatten<C: Reflect + Into<Project05Component>>(chip: C) -> IC<Computational16> {
    fn go(comp: Project05Component) -> Vec<Computational16> {
        match comp.expand() {
            None => match comp {
                Project05Component::Project03(p) =>
                    crate::project_03::flatten(p)
                        .components.into_iter()
                        .map(|s| match s {
                            Sequential::Nand(n)     => Computational::Nand(n),
                            Sequential::Const(c)    => Computational::Const(c),
                            Sequential::Buffer(c)   => Computational::Buffer(c),
                            Sequential::Mux(m)      => Computational::Mux(m),
                            Sequential::Mux1(m)     => Computational::Mux1(m),
                            Sequential::Adder(a)    => Computational::Adder(a),
                            Sequential::Register(r) => Computational::Register(r),
                        })
                        .collect(),
                Project05Component::ROM(r)          => vec![Computational::ROM(r)],
                Project05Component::MemorySystem(m) => vec![Computational::MemorySystem(m)],
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

pub const RAM_BASE:    u16 = 0 * 1024;
pub const SCREEN_BASE: u16 = 16 * 1024;
pub const KEYBOARD:    u16 = 24 * 1024;

/// Our MemorySystem: Main RAM (16KB), screen buffer (8KB), and I/O, starting from address 0.
///
/// Note: the ROM is *not* mapped into this address space; it has it's own separate connection to
/// the CPU.
pub fn memory_system() -> MemoryMap {
    MemoryMap {
        regions: vec![
            // Main memory:
            RegionMap::RAM(RAMMap {
                size: (SCREEN_BASE - RAM_BASE) as usize,
                base: RAM_BASE as usize
            }),
            // Screen buffer:
            RegionMap::RAM(RAMMap {
                size: (KEYBOARD - SCREEN_BASE) as usize,
                base: SCREEN_BASE as usize
            }),
            // "Keyboard":
            RegionMap::Serial(SerialMap {
                base: KEYBOARD as usize
            }),
        ],
    }
}

/// Access the main RAM region (base address 0) of the MemorySystem.
pub fn find_ram(state: &ChipState<N16>) -> RAMHandle {
    state.bus_residents().iter()
        .find_map(|r| if let BusResident::RAM(h) = r { if h.base == 0 { Some(h.clone()) } else { None } } else { None })
        .expect("no RAM region at base 0")
}

/// Access the screen RAM region (base address 16384) of the MemorySystem.
pub fn find_screen(state: &ChipState<N16>) -> RAMHandle {
    state.bus_residents().iter()
        .find_map(|r| if let BusResident::RAM(h) = r { if h.base == SCREEN_BASE as usize { Some(h.clone()) } else { None } } else { None })
        .expect("no RAM region at SCREEN_BASE")
}

/// Access the serial interface which is normally used to provide keyboard input to the CPU,
/// assuming a normal MemorySystem is present. Otherwise panic.
pub fn find_keyboard(state: &ChipState<N16>) -> SerialHandle {
    state.bus_residents().iter()
        .find_map(|r| if let BusResident::Serial(h) = r { Some(h.clone()) } else { None })
        .expect("no Serial device found")
}

/// Access the ROM, assuming a normal MemorySystem is present. Otherwise panic.
pub fn find_rom(state: &ChipState<N16>) -> ROMHandle {
    state.bus_residents().iter()
        .find_map(|r| if let BusResident::ROM(h) = r { Some(h.clone()) } else { None })
        .expect("no ROM found")
}

/// Strictly speaking, this *could* be pure wiring; this component just makes the unpacking of
/// instructions easier to test and to use separately.
///
/// But treating the bits of an A-instruction as control signals means the simulator has to evaluate
/// lots of meaningless signals (mostly, the ALU), so all these control lines are low when is_c is
/// low. That costs a few gates, but saves a *lot* of evaluation.
#[derive(Reflect, Chip)]
pub struct Decode {
    /// Instuction word from the ROM
    pub instr: Input16,

    /// If true (bit 15 = 1), this is a C-instruction (ALU involved);
    /// otherwise an A-instruction (load bits into A register).
    pub is_c: Output,

    /// If true, the "X" input to the ALU is the memory (M), otherwise register A.
    pub read_m: Output,

    // ALU control bits:
    pub zx: Output,
    pub nx: Output,
    pub zy: Output,
    pub ny: Output,
    pub f:  Output,
    pub no: Output,

    /// If true, write ALU output to the A register.
    pub write_a: Output,

    /// If true, write ALU output to memory at address A.
    pub write_m: Output,

    /// If true, write ALU output to the D register.
    pub write_d: Output,

    // Jump flags
    pub jmp_lt: Output,
    pub jmp_eq: Output,
    pub jmp_gt: Output,
}

impl Component for Decode {
    // Note: in fact, this is only using Buffer and And, which is only Combinational, but it keeps
    // life simple if everything in this file flattens to the same type.
    type Target = Project05Component;

    fn expand(&self) -> Option<IC<Project05Component>> {
        fn wrap<C>(comp: C) -> Project05Component
            where Project01Component: From<C>
        {
            let p01: Project01Component = comp.into();
            let p02: Project02Component = p01.into();
            let p03: Project03Component = p02.into();
            p03.into()
        }

        Some(IC {
            name: self.name().to_string(),
            intf: self.reflect(),
            components: vec![
                wrap(Buffer { a: self.instr.bit(15).clone(), out: self.is_c.clone() }),

                // Note: CPU control signalss all gated with is_c so they're false on A-instructions and this
                // simplifies the logic in CPU

                wrap(And { a: self.instr.bit(12).clone(), b: self.is_c.clone().into(), out: self.read_m.clone() }),

                // ALU control lines: mostly just buffer them through because the ALU is dealt with separately
                wrap(Buffer { a: self.instr.bit(11).clone(), out: self.zx.clone() }),
                wrap(Buffer { a: self.instr.bit(10).clone(), out: self.nx.clone() }),
                wrap(Buffer { a: self.instr.bit( 9).clone(), out: self.zy.clone() }),
                wrap(Buffer { a: self.instr.bit( 8).clone(), out: self.ny.clone() }),
                // Special-case: prefer f = 0, to bias against evaluating Add16
                wrap(And { a: self.instr.bit( 7).clone(), b: self.is_c.clone().into(), out: self.f.clone() }),
                wrap(Buffer { a: self.instr.bit( 6).clone(), out: self.no.clone() }),

                wrap(And { a: self.instr.bit( 5).clone(), b: self.is_c.clone().into(), out: self.write_a.clone() }),
                wrap(And { a: self.instr.bit( 4).clone(), b: self.is_c.clone().into(), out: self.write_d.clone() }),
                wrap(And { a: self.instr.bit( 3).clone(), b: self.is_c.clone().into(), out: self.write_m.clone() }),
                wrap(And { a: self.instr.bit( 2).clone(), b: self.is_c.clone().into(), out: self.jmp_lt.clone() }),
                wrap(And { a: self.instr.bit( 1).clone(), b: self.is_c.clone().into(), out: self.jmp_eq.clone() }),
                wrap(And { a: self.instr.bit( 0).clone(), b: self.is_c.clone().into(), out: self.jmp_gt.clone() }),
            ],
        })
    }
}

#[derive(Reflect, Chip)]
pub struct CPU {
    /// Return to a known state (i.e. jump to address 0)
    pub reset: Input,

    /// Address of the next instruction to load
    pub pc: Output16,

    /// The bits of the current instruction
    pub instr: Input16,

    pub mem_data_out: Output16,
    pub mem_write: Output,

    /// Feed-forward: address to write at the end of this cycle, and read from in the *next* cycle
    pub mem_addr: Output16,

    pub mem_data_in: Input16,
}

impl Component for CPU {
    // Note: in fact, this doesn't need the MemorySystem, but it keeps
    // life simple if everything in this file flattens to the same type.
    type Target = Project05Component;

    fn expand(&self) -> Option<IC<Project05Component>> {
        let mut components: Vec<Project05Component> = vec![];

        // === Decode ===
        let decode = Decode {
            instr:   self.instr.clone(),
            is_c:    Output::new(), read_m:  Output::new(),
            zx:      Output::new(), nx:      Output::new(),
            zy:      Output::new(), ny:      Output::new(),
            f:       Output::new(), no:      Output::new(),
            write_a: Output::new(), write_m: Output::new(), write_d: Output::new(),
            jmp_lt:  Output::new(), jmp_eq:  Output::new(), jmp_gt:  Output::new(),
        };
        let is_c        = decode.is_c.clone();        // instr[15]: 1 = C-instruction
        let read_m_out  = decode.read_m.clone();
        let zx          = decode.zx.clone();
        let nx          = decode.nx.clone();
        let zy          = decode.zy.clone();
        let ny          = decode.ny.clone();
        let f           = decode.f.clone();
        let no          = decode.no.clone();
        let dec_write_a = decode.write_a.clone();
        let dec_write_m = decode.write_m.clone();
        let dec_write_d = decode.write_d.clone();
        let jmp_lt      = decode.jmp_lt.clone();
        let jmp_eq      = decode.jmp_eq.clone();
        let jmp_gt      = decode.jmp_gt.clone();
        components.push(Project05Component::Decode(decode));

        // === is_a = NOT(is_c) ===
        let is_a_gate = Not { a: is_c.clone().into(), out: Output::new() };
        let is_a = is_a_gate.out.clone();
        components.push(p01(is_a_gate));

        // Declare a_data wire up-front; driven by a_data_mux placed AFTER the ALU so that
        // the second Nand pass sees the correct ALU output (fixes A=M indirect addressing).
        let a_data = Output16::new();
        let a_out  = Output16::new();  // A register output (internal wire)

        // === load_a = is_a OR write_a ===
        let load_a_gate = Or { a: is_a.clone().into(), b: dec_write_a.into(), out: Output::new() };
        let load_a = load_a_gate.out.clone();
        components.push(p01(load_a_gate));

        // === A register: out → mem_addr ===
        let reg_a = Register16 { data_in: a_data.clone().into(), write: load_a.clone().into(), data_out: a_out.clone() };
        components.push(p03(reg_a));

        // === ALU Y mux: sel=read_m → a0=A (mem_addr), a1=mem_in ===
        let y_mux = Mux16 {
            sel: read_m_out.into(),
            a0:  a_out.clone().into(),
            a1:  self.mem_data_in.clone(),
            out: Output16::new(),
        };
        let y_src = y_mux.out.clone();
        components.push(p01(y_mux));

        // === ALU: x=D, y=y_src, enabled only on C-instructions ===
        let reg_d_out: Output16 = Output16::new();  // D register output wire (seeded from reg_state)
        let alu = ALU {
            x:   reg_d_out.clone().into(),
            y:   y_src.into(),
            zx:  zx.into(), nx: nx.into(),
            zy:  zy.into(), ny: ny.into(),
            f:   f.into(),  no: no.into(),
            disable: is_a.clone().into(),
            out: self.mem_data_out.clone(),
            zr:  Output::new(),
            ng:  Output::new(),
        };
        let alu_zr = alu.zr.clone();
        let alu_ng = alu.ng.clone();
        components.push(p02(alu));

        // === A register data mux: AFTER ALU ===
        // sel=is_a → a1=instr (A-instr), a0=ALU output (C-instr with dest=A)
        let a_data_mux = Mux16 {
            sel: is_a.into(),
            a0:  self.mem_data_out.clone().into(),
            a1:  self.instr.clone(),
            out: a_data.clone(),
        };
        components.push(p01(a_data_mux));

        // === next_addr: if A is being written this cycle, expose the new A value as the
        // address for the memory system (so RAM latches the right read address); otherwise
        // expose the current A.out. Write address is always A.out (load_a=0 when write_m=1). ===
        let next_addr_mux = Mux16 {
            sel: load_a.clone().into(),
            a0:  a_out.clone().into(),
            a1:  a_data.into(),
            out: self.mem_addr.clone(),
        };
        components.push(p01(next_addr_mux));

        // === D register (write_d already gated with is_c in Decode) ===
        let reg_d = Register16 { data_in: self.mem_data_out.clone().into(), write: dec_write_d.into(), data_out: reg_d_out };
        components.push(p03(reg_d));

        // === mem_write (write_m already gated with is_c in Decode) ===
        components.push(p01(Buffer { a: dec_write_m.into(), out: self.mem_write.clone() }));

        // === Jump logic ===
        let not_ng  = Not { a: alu_ng.clone().into(), out: Output::new() };
        let not_zr  = Not { a: alu_zr.clone().into(), out: Output::new() };
        let is_pos  = And { a: not_ng.out.clone().into(), b: not_zr.out.clone().into(), out: Output::new() };
        // Jump signals already gated with is_c in Decode.
        let jlt_and = And { a: jmp_lt.into(), b: alu_ng.into(), out: Output::new() };
        let jeq_and = And { a: jmp_eq.into(), b: alu_zr.into(), out: Output::new() };
        let jgt_and = And { a: jmp_gt.into(), b: is_pos.out.clone().into(), out: Output::new() };
        let j_lt_eq = Or  { a: jlt_and.out.clone().into(), b: jeq_and.out.clone().into(), out: Output::new() };
        let jump_any= Or  { a: j_lt_eq.out.clone().into(), b: jgt_and.out.clone().into(), out: Output::new() };
        let do_jump_out = jump_any.out.clone();
        for g in [p01(not_ng), p01(not_zr), p01(is_pos),
                  p01(jlt_and), p01(jeq_and), p01(jgt_and),
                  p01(j_lt_eq), p01(jump_any)] {
            components.push(g);
        }

        // === PC: inc always 1 ===
        let const_one = Const { value: 1, out: Output::new() };
        let const_one_out = const_one.out.bit(0).into();
        components.push(p01(const_one));

        let pc = PC {
            addr:  a_out.clone().into(),
            load:  do_jump_out.into(),
            inc:   const_one_out,
            reset: self.reset.clone().into(),
            out:   self.pc.clone(),
        };
        components.push(p03(pc));

        Some(IC { name: self.name().to_string(), intf: self.reflect(), components })
    }
}

#[derive(Reflect, Chip)]
pub struct Computer {
    /// A way to force the CPU to return to a known state (i.e. jump to address 0)
    pub reset: Input,

    /// Useful for debugging, but also acts as a root for traversing the graph
    pub pc: Output16,
    // TODO: tty_ready?
}

impl Component for Computer {
    type Target = Project05Component;

    /*
      let cpu = CPU { reset: self.reset, instr: rom.out, mem_in: memory.out, pc: self.pc, mem_out, mem_write, mem_addr }
      let rom = ROM16 { size: 32K, addr: cpu.pc }
      let memory = MemorySystem { data: cpu.mem_data_out, load: cpu.mem_write, addr: cpu.mem_addr }
      outputs.pc = cpu.pc
     */
    fn expand(&self) -> Option<IC<Project05Component>> {
        let mem_data_in_wire = Output16::new();  // back-ref from memory to CPU

        let rom = ROM16 {
            size: 32 * 1024,
            addr: self.pc.clone().into(),
            out:  Output16::new(),
        };

        let cpu = CPU {
            reset:     self.reset.clone(),
            pc:        self.pc.clone(),
            instr:     rom.out.clone().into(),
            mem_data_out:   Output16::new(),
            mem_write: Output::new(),
            mem_addr: Output16::new(),
            mem_data_in: mem_data_in_wire.clone().into(),
        };

        let memory = MemorySystem16 {
            addr:     cpu.mem_addr.clone().into(),
            write:    cpu.mem_write.clone().into(),
            data_in:  cpu.mem_data_out.clone().into(),
            data_out: mem_data_in_wire,
        };

        Some(IC {
            name: self.name().to_string(),
            intf: self.reflect(),
            components: vec![
                Project05Component::ROM(rom),
                Project05Component::CPU(cpu),
                Project05Component::MemorySystem(memory),
            ],
        })
    }
}
