#![allow(unused_variables, dead_code, unused_imports)]

use simulator::{self, Component, IC, Input, Input16, Output, Output16, Reflect, AsConst, Chip};
use simulator::Reflect as _;
use simulator::Chip as _;
use simulator::component::{Nand, Register16, RAM16, ROM16, MemorySystem16, Sequential, Computational, Computational16};
use simulator::simulate::{ChipState, BusResident, RAMHandle, ROMHandle, MemoryMap, RAMMap, RegionHandle};
use crate::project_01::{Project01Component, Not, And, Or, Mux16};
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

/// Our MemorySystem: Main RAM (16KB), screen buffer (8KB), and I/O, starting from address 0.
pub fn memory_system() -> MemoryMap {
    MemoryMap::new(vec![
        RAMMap { size: 16*1024, base: 0 },
        RAMMap { size:  8*1024, base: 16*1024 },
        // RAMMap for keyboard at 24*1024 when needed
    ])
}

// /// Main RAM (16KB), screen buffer (8KB), and I/O.
// ///
// /// During simulation, these components are exposed as BusResidents.
// ///
// /// TODO: this logic will be external to the CPU, which will assume a simple, flat memory model.
// /// Essentially, all the CPU knows is there is a memory which it can read and write, with an
// /// address size of 15 bits.
// ///
// /// If it's interesting to do this decoding in simulated circuitry, then this decode logic
// /// will get separately simulated within a component that plugs into the simulated CPU. To get
// /// started, it will just be handled by native code with some customization, because it's just not
// /// that interesting... all HACK-family CPUs will share the same memory layout, and memory system
// /// performance isn't what this project is about.
// #[derive(Reflect, Chip)]
// pub struct MemorySystem {
//     pub data: Input16,
//     pub load: Input,
//     pub addr: Input16,

//     pub out: Output16,
//     // TODO: tty_ready?
// }

// impl Component for MemorySystem {
//     type Target = Project05Component;

//     fn expand(&self) -> Option<IC<Project05Component>> {
//         use simulator::Input16;
//         let mut components: Vec<Project05Component> = vec![];

//         // addr[14]=1 → screen/keyboard range; addr[15]=1 → out of range
//         let sel_screen = self.addr.bit(14);
//         let sel_oor    = self.addr.bit(15);

//         let not_screen_gate = Not { a: sel_screen.clone().into(), out: Output::new() };
//         let not_screen = not_screen_gate.out.clone();
//         components.push(p01(not_screen_gate));

//         let not_oor_gate = Not { a: sel_oor.clone().into(), out: Output::new() };
//         let not_oor = not_oor_gate.out.clone();
//         components.push(p01(not_oor_gate));

//         // load_valid = AND(self.load, NOT(addr[15]))
//         let load_valid_gate = And { a: self.load.clone().into(), b: not_oor.into(), out: Output::new() };
//         let load_valid = load_valid_gate.out.clone();
//         components.push(p01(load_valid_gate));

//         // load_ram    = AND(load_valid, NOT(addr[14]))
//         let load_ram_gate = And { a: load_valid.clone().into(), b: not_screen.into(), out: Output::new() };
//         let load_ram = load_ram_gate.out.clone();
//         components.push(p01(load_ram_gate));

//         // load_screen = AND(load_valid, addr[14])
//         let load_screen_gate = And { a: load_valid.into(), b: sel_screen.clone().into(), out: Output::new() };
//         let load_screen = load_screen_gate.out.clone();
//         components.push(p01(load_screen_gate));

//         // Main RAM (16KB): addr bits 0-13 (14-bit addressing)
//         let ram = RAM16 {
//             size: 16 * 1024,
//             addr: self.addr.mask(0, 14),
//             data: self.data.clone(),
//             load: load_ram.into(),
//             out: Output16::new(),
//         };
//         let ram_out = ram.out.clone();
//         components.push(Project05Component::RAM(ram));

//         // Screen buffer (8KB): addr bits 0-12 (13-bit addressing)
//         let screen = RAM16 {
//             size: 8 * 1024,
//             addr: self.addr.mask(0, 13),
//             data: self.data.clone(),
//             load: load_screen.into(),
//             out: Output16::new(),
//         };
//         let screen_out = screen.out.clone();
//         components.push(Project05Component::RAM(screen));

//         // Inner mux: sel=addr[14] → a0=ram_out (RAM), a1=screen_out (Screen)
//         let inner_mux = Mux16 {
//             sel: sel_screen.into(),
//             a0:  ram_out.into(),
//             a1:  screen_out.into(),
//             out: Output16::new(),
//         };
//         let inner_out = inner_mux.out.clone();
//         components.push(p01(inner_mux));

//         // Outer mux: sel=addr[15] → a0=valid data, a1=0 (out-of-range)
//         let outer_mux = Mux16 {
//             sel: sel_oor.into(),
//             a0:  inner_out.into(),
//             a1:  Input16::new(),  // undriven = constant 0
//             out: self.out.clone(),
//         };
//         components.push(p01(outer_mux));

//         Some(IC { name: self.name().to_string(), intf: self.reflect(), components })
//     }
// }

pub const RAM_BASE:    u16 = 0 * 1024;
pub const SCREEN_BASE: u16 = 16 * 1024;
pub const KEYBOARD:    u16 = 24 * 1024;

/// Access the main RAM region (base address 0) of the MemorySystem.
pub fn find_ram(state: &ChipState) -> RegionHandle {
    state.bus_residents().iter()
        .find_map(|r| if let BusResident::MemorySystem(h) = r { Some(RegionHandle::new(h.clone(), 0)) } else { None })
        .expect("no MemorySystem found")
}

/// Access the screen RAM region (base address 16384) of the MemorySystem.
pub fn find_screen(state: &ChipState) -> RegionHandle {
    state.bus_residents().iter()
        .find_map(|r| if let BusResident::MemorySystem(h) = r { Some(RegionHandle::new(h.clone(), SCREEN_BASE.into())) } else { None })
        .expect("no MemorySystem found")
}

/// Access the ROM, assuming a normal MemorySystem is present. Otherwise panic.
pub fn find_rom(state: &ChipState) -> ROMHandle {
    state.bus_residents().iter()
        .find_map(|r| if let BusResident::ROM(h) = r { Some(h.clone()) } else { None })
        .expect("no ROM found")
}

/// Pure wiring; this component just makes the unpacking of instructions easier to test and
/// to use separately.
///
/// Note: due to the deficient way this kind of wiring is currently handled, it would be better
/// at the moment to express this another way, but this is probably the right way to go eventually.
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
    // Note: in fact, this is only using Nots and really shouldn't even need that, but it keeps
    // life simple if everything in this file flattens to the same type.
    type Target = Project05Component;

    fn expand(&self) -> Option<IC<Project05Component>> {
        let mut components: Vec<Project05Component> = vec![];

        fn wrap(not: Not) -> Project05Component {
            let p01: Project01Component = not.into();
            let p02: Project02Component = p01.into();
            let p03: Project03Component = p02.into();
            p03.into()
        }
        let mut wire = |src: Input, dst: Output| {
            // NOT(NOT(src)) = src: a dumb way to express a plain wire using Nand gates.
            // This is a workaround because there's currently no way to express wiring without some
            // component.
            let mid  = Not { a: src, out: Not::chip().out };
            let pass = Not { a: mid.out.clone().into(), out: dst };
            for not in [mid, pass] { components.push(wrap(not)); }
        };

        wire(self.instr.bit(15).clone(), self.is_c.clone());
        // bit-14: unused
        // bit-13: unused
        wire(self.instr.bit(12).clone(), self.read_m.clone());

        wire(self.instr.bit(11).clone(), self.zx.clone());
        wire(self.instr.bit(10).clone(), self.nx.clone());
        wire(self.instr.bit( 9).clone(), self.zy.clone());
        wire(self.instr.bit( 8).clone(), self.ny.clone());
        wire(self.instr.bit( 7).clone(), self.f.clone());
        wire(self.instr.bit( 6).clone(), self.no.clone());

        wire(self.instr.bit( 5).clone(), self.write_a.clone());
        wire(self.instr.bit( 4).clone(), self.write_d.clone());
        wire(self.instr.bit( 3).clone(), self.write_m.clone());

        wire(self.instr.bit( 2).clone(), self.jmp_lt.clone());
        wire(self.instr.bit( 1).clone(), self.jmp_eq.clone());
        wire(self.instr.bit( 0).clone(), self.jmp_gt.clone());

        Some(IC {
            name: self.name().to_string(),
            intf: self.reflect(),
            components,
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

        // === ALU: x=D, y=y_src, out → mem_out ===
        let reg_d_out: Output16 = Output16::new();  // D register output wire (seeded from reg_state)
        let alu = ALU {
            x:   reg_d_out.clone().into(),
            y:   y_src.into(),
            zx:  zx.into(), nx: nx.into(),
            zy:  zy.into(), ny: ny.into(),
            f:   f.into(),  no: no.into(),
            out: self.mem_data_out.clone(),
            zr:  Output::new(),
            ng:  Output::new(),
        };
        let alu_zr = alu.zr.clone();
        let alu_ng = alu.ng.clone();
        components.push(p02(alu));

        // === A register data mux: AFTER ALU so pass 2 reads correct ALU output ===
        // sel=is_a → a1=instr (A-instr), a0=alu_out (C-instr with dest=A)
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

        // === load_d = AND(is_c, write_d) ===
        let load_d_gate = And { a: is_c.clone().into(), b: dec_write_d.into(), out: Output::new() };
        let load_d = load_d_gate.out.clone();
        components.push(p01(load_d_gate));

        // === D register ===
        let reg_d = Register16 { data_in: self.mem_data_out.clone().into(), write: load_d.into(), data_out: reg_d_out };
        components.push(p03(reg_d));

        // === mem_write = AND(is_c, write_m) ===
        components.push(p01(And { a: is_c.clone().into(), b: dec_write_m.into(), out: self.mem_write.clone() }));

        // === Jump logic ===
        let not_ng  = Not { a: alu_ng.clone().into(), out: Output::new() };
        let not_zr  = Not { a: alu_zr.clone().into(), out: Output::new() };
        let is_pos  = And { a: not_ng.out.clone().into(), b: not_zr.out.clone().into(), out: Output::new() };
        let jlt_and = And { a: jmp_lt.into(), b: alu_ng.into(), out: Output::new() };
        let jeq_and = And { a: jmp_eq.into(), b: alu_zr.into(), out: Output::new() };
        let jgt_and = And { a: jmp_gt.into(), b: is_pos.out.clone().into(), out: Output::new() };
        let j_lt_eq = Or  { a: jlt_and.out.clone().into(), b: jeq_and.out.clone().into(), out: Output::new() };
        let jump_any= Or  { a: j_lt_eq.out.clone().into(), b: jgt_and.out.clone().into(), out: Output::new() };
        let do_jump = And { a: is_c.clone().into(), b: jump_any.out.clone().into(), out: Output::new() };
        let do_jump_out = do_jump.out.clone();
        for g in [p01(not_ng), p01(not_zr), p01(is_pos),
                  p01(jlt_and), p01(jeq_and), p01(jgt_and),
                  p01(j_lt_eq), p01(jump_any), p01(do_jump)] {
            components.push(g);
        }

        // === PC: inc always 1 (NAND of two undriven=0 inputs) ===
        let const_one = Nand { a: Input::new(), b: Input::new(), out: Output::new() };
        let inc_wire  = const_one.out.clone();
        components.push(p01(const_one));

        let pc = PC {
            addr:  a_out.clone().into(),
            load:  do_jump_out.into(),
            inc:   inc_wire.into(),
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
