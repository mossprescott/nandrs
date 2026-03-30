#![allow(unused_variables, dead_code, unused_imports)]

use crate::project_01::{And, Mux16, Not, Or, Project01Component};
use crate::project_02::{ALU, Project02Component};
use crate::project_03::{PC, Project03Component};
use simulator::Chip as _;
use simulator::Reflect as _;
use simulator::component::native;
use simulator::component::{
    Buffer, Computational, Computational16, MemorySystem16, Nand, RAM16, ROM16, Register16,
    Sequential,
};
use simulator::declare::{BusRef, Interface};
use simulator::nat::N16;
use simulator::simulate::{
    BusResident, ChipState, MemoryMap, RAMHandle, RAMMap, ROMHandle, ROMMap, RegionMap,
    SerialHandle, SerialMap,
};
use simulator::word::Word16;
use simulator::{
    self, Chip, Component, IC, Input1, Input16, Output, Output16, Reflect, expand, fixed,
};

#[derive(Clone, Reflect, Component)]
pub enum Project05Component {
    #[delegate]
    Project03(Project03Component),
    #[primitive]
    ROM(ROM16),
    #[primitive]
    MemorySystem(MemorySystem16),
    Decode(Decode),
    CPU(CPU),
    Computer(Computer),
}

/// Recursively expand until only Nands, Registers, RAMs, and ROMs are left.
pub fn flatten<C: Reflect + Into<Project05Component>>(chip: C) -> IC<Computational16> {
    fn go(comp: Project05Component) -> Vec<Computational16> {
        match comp.expand() {
            None => match comp {
                Project05Component::Project03(p) => crate::project_03::flatten(p)
                    .components
                    .into_iter()
                    .map(|s| match s {
                        Sequential::Nand(n) => Computational::Nand(n),
                        Sequential::Buffer(c) => Computational::Buffer(c),
                        Sequential::Register(r) => Computational::Register(r),
                    })
                    .collect(),
                Project05Component::ROM(r) => vec![Computational::ROM(r)],
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

/// Like `flatten`, but replaces FullAdder with native Adder for efficient simulation.
pub fn flatten_for_simulation<C: Reflect + Into<Project05Component>>(
    chip: C,
) -> IC<native::Simulational<N16, N16>> {
    fn go(comp: Project05Component) -> Vec<native::Simulational<N16, N16>> {
        // Delegate to lower-level flatten_for_simulation as soon as possible:
        match comp {
            Project05Component::Project03(Project03Component::Project02(p)) => {
                return crate::project_02::flatten_for_simulation(p).components;
            }
            Project05Component::Project03(Project03Component::Register(r)) => {
                return vec![Computational::Register(r.into()).into()];
            }
            Project05Component::ROM(r) => return vec![Computational::ROM(r).into()],
            Project05Component::MemorySystem(m) => {
                return vec![Computational::MemorySystem(m).into()];
            }
            _ => {}
        }
        match comp.expand() {
            Some(ic) => ic.components.into_iter().flat_map(go).collect(),
            None => panic!("Did not reduce to primitive: {:?}", comp.name()),
        }
    }
    IC {
        name: format!("{} (flat/sim)", chip.name()),
        intf: chip.reflect(),
        components: go(chip.into()),
    }
}

pub const RAM_BASE: u16 = 0 * 1024;
pub const SCREEN_BASE: u16 = 16 * 1024;
pub const KEYBOARD: u16 = 24 * 1024;

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
                base: RAM_BASE as usize,
            }),
            // Screen buffer:
            RegionMap::RAM(RAMMap {
                size: (KEYBOARD - SCREEN_BASE) as usize,
                base: SCREEN_BASE as usize,
            }),
            // "Keyboard":
            RegionMap::Serial(SerialMap {
                base: KEYBOARD as usize,
            }),
        ],
    }
}

/// Access the main RAM region (base address 0) of the MemorySystem.
pub fn find_ram(state: &ChipState<N16, N16>) -> RAMHandle<N16, N16> {
    state
        .bus_residents()
        .iter()
        .find_map(|r| {
            if let BusResident::RAM(h) = r {
                if h.base == 0 { Some(h.clone()) } else { None }
            } else {
                None
            }
        })
        .expect("no RAM region at base 0")
}

/// Access the screen RAM region (base address 16384) of the MemorySystem.
pub fn find_screen(state: &ChipState<N16, N16>) -> RAMHandle<N16, N16> {
    state
        .bus_residents()
        .iter()
        .find_map(|r| {
            if let BusResident::RAM(h) = r {
                if h.base == SCREEN_BASE as usize {
                    Some(h.clone())
                } else {
                    None
                }
            } else {
                None
            }
        })
        .expect("no RAM region at SCREEN_BASE")
}

/// Access the serial interface which is normally used to provide keyboard input to the CPU,
/// assuming a normal MemorySystem is present. Otherwise panic.
pub fn find_keyboard(state: &ChipState<N16, N16>) -> SerialHandle<N16> {
    state
        .bus_residents()
        .iter()
        .find_map(|r| {
            if let BusResident::Serial(h) = r {
                Some(h.clone())
            } else {
                None
            }
        })
        .expect("no Serial device found")
}

/// Access the ROM, assuming a normal MemorySystem is present. Otherwise panic.
pub fn find_rom(state: &ChipState<N16, N16>) -> ROMHandle<N16, N16> {
    state
        .bus_residents()
        .iter()
        .find_map(|r| {
            if let BusResident::ROM(h) = r {
                Some(h.clone())
            } else {
                None
            }
        })
        .expect("no ROM found")
}

/// Strictly speaking, this *could* be pure wiring; this component just makes the unpacking of
/// instructions easier to test and to use separately.
///
/// But treating the bits of an A-instruction as control signals means the simulator has to evaluate
/// lots of meaningless signals (mostly, the ALU), so all these control lines are low when is_c is
/// low. That costs a few gates, but saves a *lot* of evaluation.
#[derive(Clone, Reflect, Chip)]
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
    pub f: Output,
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

#[derive(Clone, Reflect, Chip)]
pub struct CPU {
    /// Return to a known state (i.e. jump to address 0)
    pub reset: Input1,

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

    expand! { |this| {
        // Forward-declare register outputs:
        reg_a_out: forward Output16::new(),
        reg_d_out: forward Output16::new(),

        // === Decode ===
        decode: Decode {
            instr: this.instr,

            is_c: Output::new(),
            is_a: Output::new(),

            read_m: Output::new(),

            zx: Output::new(), nx: Output::new(),
            zy: Output::new(), ny: Output::new(),
            f:  Output::new(), no: Output::new(),

            write_a: Output::new(), write_m: Output::new(), write_d: Output::new(),

            jmp_lt:  Output::new(), jmp_eq:  Output::new(), jmp_gt:  Output::new(),
        },

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
            a1:  this.instr,
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
        jump_any: Or  { a: j_lt_eq.out.into(), b: jgt_and.out.into(), out: Output::new() },

        // === PC: inc always 1 ===
        pc: PC {
            addr:  reg_a_out.into(),
            load:  jump_any.out.into(),
            inc:   fixed(1),
            reset: this.reset.into(),
            out:   this.pc,
        },
    }}
}

#[derive(Clone, Reflect, Chip)]
pub struct Computer {
    /// A way to force the CPU to return to a known state (i.e. jump to address 0)
    pub reset: Input1,

    /// Useful for debugging, but also acts as a root for traversing the graph
    pub pc: Output16,
}

impl Component for Computer {
    type Target = Project05Component;

    expand! { |this| {
        mem_out: forward Output16::new(),

        rom: ROM16 {
            size: 32 * 1024,
            addr: this.pc.into(),
            out:  Output16::new(),
        },

        cpu: CPU {
            reset:        this.reset,
            pc:           this.pc,
            instr:        rom.out.into(),
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
