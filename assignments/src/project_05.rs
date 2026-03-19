#![allow(unused_variables, dead_code, unused_imports)]

use simulator::{self, Component, IC, Input, Input16, Output, Output16, Reflect, AsConst, Chip, expand};
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
    // Previous project:
    Project03(Project03Component),

    // Memory primitives:
    ROM(ROM16),
    MemorySystem(MemorySystem16),

    // New here:
    Decode(Decode),
    CPU(CPU),
    Computer(Computer),
}

impl<C: Into<Project03Component>> From<C> for Project05Component {
    fn from(c: C) -> Self {
        Project05Component::Project03(c.into())
    }
}
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
pub fn find_ram(state: &ChipState<N16, N16>) -> RAMHandle<N16, N16> {
    state.bus_residents().iter()
        .find_map(|r| if let BusResident::RAM(h) = r { if h.base == 0 { Some(h.clone()) } else { None } } else { None })
        .expect("no RAM region at base 0")
}

/// Access the screen RAM region (base address 16384) of the MemorySystem.
pub fn find_screen(state: &ChipState<N16, N16>) -> RAMHandle<N16, N16> {
    state.bus_residents().iter()
        .find_map(|r| if let BusResident::RAM(h) = r { if h.base == SCREEN_BASE as usize { Some(h.clone()) } else { None } } else { None })
        .expect("no RAM region at SCREEN_BASE")
}

/// Access the serial interface which is normally used to provide keyboard input to the CPU,
/// assuming a normal MemorySystem is present. Otherwise panic.
pub fn find_keyboard(state: &ChipState<N16, N16>) -> SerialHandle<N16> {
    state.bus_residents().iter()
        .find_map(|r| if let BusResident::Serial(h) = r { Some(h.clone()) } else { None })
        .expect("no Serial device found")
}

/// Access the ROM, assuming a normal MemorySystem is present. Otherwise panic.
pub fn find_rom(state: &ChipState<N16, N16>) -> ROMHandle<N16, N16> {
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

    /// Inverse of is_c.
    pub is_a: Output,

    /// If true, the "X" input to the ALU is the memory (M), otherwise register A.
    pub read_m: Output,

    // ALU control bits:
    pub zx: Output,
    pub nx: Output,
    pub zy: Output,
    pub ny: Output,
    pub f:  Output,
    pub no: Output,

    /// If true, write ALU output to the A register (where it will appear in the next cycle).
    pub write_a: Output,

    /// If true, write ALU output to memory at address A (the value as of this cycle).
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

    expand! { |this| {
        // TODO: buffers don't need to be named. Or declared in this way at all, maybe?
        _15: Buffer { a: this.instr.bit(15), out: this.is_c },

        _is_a: Not { a: this.instr.bit(15), out: this.is_a },

        // _14: unused/reserved
        // _13: unused/reserved

        // Note: CPU control signals all gated with is_c so they're false on A-instructions and this
        // simplifies the logic in CPU

        _12: And    { a: this.instr.bit(12), b: this.is_c.into(), out: this.read_m },

        // ALU control lines: mostly just buffer them through because the ALU is dealt with separately
        _11: Buffer { a: this.instr.bit(11), out: this.zx },
        _10: Buffer { a: this.instr.bit(10), out: this.nx },
        _9:  Buffer { a: this.instr.bit( 9), out: this.zy },
        _8:  Buffer { a: this.instr.bit( 8), out: this.ny },
        // Special-case: prefer f = 0, to bias against evaluating Add16
        _7:  And    { a: this.instr.bit( 7), b: this.is_c.into(), out: this.f },
        _6:  Buffer { a: this.instr.bit( 6), out: this.no },

        _5:  And    { a: this.instr.bit( 5), b: this.is_c.into(), out: this.write_a },
        _4:  And    { a: this.instr.bit( 4), b: this.is_c.into(), out: this.write_d },
        _3:  And    { a: this.instr.bit( 3), b: this.is_c.into(), out: this.write_m },
        _2:  And    { a: this.instr.bit( 2), b: this.is_c.into(), out: this.jmp_lt },
        _1:  And    { a: this.instr.bit( 1), b: this.is_c.into(), out: this.jmp_eq },
        _0:  And    { a: this.instr.bit( 0), b: this.is_c.into(), out: this.jmp_gt },
    }}
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
        // === Decode ===
        let decode = Decode {
            instr: self.instr,

            is_c: Output::new(),
            is_a: Output::new(),

            read_m: Output::new(),

            zx: Output::new(), nx: Output::new(),
            zy: Output::new(), ny: Output::new(),
            f:  Output::new(), no: Output::new(),

            write_a: Output::new(), write_m: Output::new(), write_d: Output::new(),

            jmp_lt:  Output::new(), jmp_eq:  Output::new(), jmp_gt:  Output::new(),
        };

        // Declare a_data wire up-front; driven by a_data_mux placed AFTER the ALU so that
        // the second Nand pass sees the correct ALU output (fixes A=M indirect addressing).
        let a_data_wire = Output16::new();
        let a_out = Output16::new();  // A register output (internal wire)

        // === load_a = is_a OR write_a ===
        let load_a = Or { a: decode.is_a.into(), b: decode.write_a.into(), out: Output::new() };

        // === A register: out → mem_addr ===
        let reg_a = Register16 { data_in: a_data_wire.into(), write: load_a.out.into(), data_out: a_out };

        // === ALU Y mux: sel=read_m → a0=A (mem_addr), a1=mem_in ===
        let y_src = Mux16 {
            sel: decode.read_m.into(),
            a0:  a_out.into(),
            a1:  self.mem_data_in,
            out: Output16::new(),
        };

        // === ALU: x=D, y=y_src, enabled only on C-instructions ===
        let reg_d_out: Output16 = Output16::new();  // D register output wire (seeded from reg_state)
        let alu = ALU {
            x:   reg_d_out.into(),
            y:   y_src.out.into(),
            zx:  decode.zx.into(), nx: decode.nx.into(),
            zy:  decode.zy.into(), ny: decode.ny.into(),
            f:   decode.f.into(),  no: decode.no.into(),
            disable: decode.is_a.into(),
            out: self.mem_data_out,
            zr:  Output::new(),
            ng:  Output::new(),
        };

        // === A register data mux: AFTER ALU ===
        // sel=is_a → a1=instr (A-instr), a0=ALU output (C-instr with dest=A)
        let a_data = Mux16 {
            sel: decode.is_a.into(),
            a0:  self.mem_data_out.into(),
            a1:  self.instr,
            out: a_data_wire,
        };

        // === next_addr: if A is being written this cycle, expose the new A value as the
        // address for the memory system (so RAM latches the right read address); otherwise
        // expose the current A.out. Write address is always A.out (load_a=0 when write_m=1). ===
        let next_addr = Mux16 {
            sel: load_a.out.into(),
            a0:  a_out.into(),
            a1:  a_data.out.into(),
            out: self.mem_addr,
        };

        // === D register (write_d already gated with is_c in Decode) ===
        let reg_d = Register16 { data_in: self.mem_data_out.into(), write: decode.write_d.into(), data_out: reg_d_out };

        // === mem_write (write_m already gated with is_c in Decode) ===
        let mem_write_buf = Buffer { a: decode.write_m.into(), out: self.mem_write };

        // === Jump logic ===
        let not_ng  = Not { a: alu.ng.into(), out: Output::new() };
        let not_zr  = Not { a: alu.zr.into(), out: Output::new() };
        let is_pos  = And { a: not_ng.out.into(), b: not_zr.out.into(), out: Output::new() };
        // Jump signals already gated with is_c in Decode.
        let jlt_and  = And { a: decode.jmp_lt.into(), b: alu.ng.into(), out: Output::new() };
        let jeq_and  = And { a: decode.jmp_eq.into(), b: alu.zr.into(), out: Output::new() };
        let jgt_and  = And { a: decode.jmp_gt.into(), b: is_pos.out.into(), out: Output::new() };
        let j_lt_eq  = Or  { a: jlt_and.out.into(), b: jeq_and.out.into(), out: Output::new() };
        let jump_any = Or  { a: j_lt_eq.out.into(), b: jgt_and.out.into(), out: Output::new() };

        // === PC: inc always 1 ===
        let const_one = Const { value: 1, out: Output::new() };

        let pc = PC {
            addr:  a_out.into(),
            load:  jump_any.out.into(),
            inc:   const_one.out.bit(0).into(),
            reset: self.reset.into(),
            out:   self.pc,
        };

        Some(IC {
            name: self.name().to_string(),
            intf: self.reflect(),
            components: vec![
                decode.into(),
                load_a.into(),
                reg_a.into(),
                y_src.into(),
                alu.into(),
                a_data.into(),
                next_addr.into(),
                reg_d.into(),
                mem_write_buf.into(),
                not_ng.into(), not_zr.into(), is_pos.into(),
                jlt_and.into(), jeq_and.into(), jgt_and.into(),
                j_lt_eq.into(), jump_any.into(),
                const_one.into(),
                pc.into(),
            ]
        })
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
