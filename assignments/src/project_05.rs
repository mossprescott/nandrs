#![allow(unused_variables, dead_code, unused_imports)]

use simulator::{self, Component, IC, Input, Input16, Output, Output16, Reflect, Chip};
use simulator::Reflect as _;
use simulator::Chip as _;
use simulator::component::{Nand, Register16, RAM16, ROM16, Sequential, Computational, Computational16};
use crate::project_01::{Project01Component, Not};
use crate::project_02::Project02Component;
use crate::project_03::Project03Component;

pub enum Project05Component {
    Project03(Project03Component),
    RAM(RAM16),
    ROM(ROM16),
    Decode(Decode),
    MemorySystem(MemorySystem),
    CPU(CPU),
    Computer(Computer),
}

impl From<Project03Component> for Project05Component { fn from(c: Project03Component) -> Self { Project05Component::Project03(c) } }
impl From<RAM16>              for Project05Component { fn from(c: RAM16)              -> Self { Project05Component::RAM(c)         } }
impl From<ROM16>              for Project05Component { fn from(c: ROM16)              -> Self { Project05Component::ROM(c)         } }
impl From<Decode>             for Project05Component { fn from(c: Decode)             -> Self { Project05Component::Decode(c)       } }
impl From<MemorySystem>       for Project05Component { fn from(c: MemorySystem)       -> Self { Project05Component::MemorySystem(c) } }
impl From<CPU>                for Project05Component { fn from(c: CPU)                -> Self { Project05Component::CPU(c)     } }
impl From<Computer>           for Project05Component { fn from(c: Computer)           -> Self { Project05Component::Computer(c) } }

impl Component for Project05Component {
    type Target = Project05Component;

    fn expand(&self) -> Option<IC<Project05Component>> {
        match self {
            Project05Component::Project03(c)    => c.expand().map(|ic| IC { name: ic.name, intf: ic.intf, components: ic.components.into_iter().map(Into::into).collect() }),
            Project05Component::RAM(c)          => c.expand().map(|_| unreachable!()),
            Project05Component::ROM(c)          => c.expand().map(|_| unreachable!()),
            Project05Component::Decode(c)       => c.expand(),
            Project05Component::MemorySystem(c) => c.expand(),
            Project05Component::CPU(c)          => c.expand(),
            Project05Component::Computer(c)     => c.expand(),
        }
    }
}

impl Reflect for Project05Component {
    fn reflect(&self) -> simulator::Interface {
        match self {
            Project05Component::Project03(c)    => c.reflect(),
            Project05Component::RAM(c)          => c.reflect(),
            Project05Component::ROM(c)          => c.reflect(),
            Project05Component::Decode(c)       => c.reflect(),
            Project05Component::MemorySystem(c) => c.reflect(),
            Project05Component::CPU(c)          => c.reflect(),
            Project05Component::Computer(c)     => c.reflect(),
        }
    }
    fn name(&self) -> &str {
        match self {
            Project05Component::Project03(c)    => c.name(),
            Project05Component::RAM(c)          => c.name(),
            Project05Component::ROM(c)          => c.name(),
            Project05Component::Decode(c)       => c.name(),
            Project05Component::MemorySystem(c) => c.name(),
            Project05Component::CPU(c)          => c.name(),
            Project05Component::Computer(c)     => c.name(),
        }
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
                            Sequential::Register(r) => Computational::Register(r),
                        })
                        .collect(),
                Project05Component::RAM(r) => vec![Computational::RAM(r)],
                Project05Component::ROM(r) => vec![Computational::ROM(r)],
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

#[derive(Reflect, Chip)]
pub struct MemorySystem {
    pub data: Input16,
    pub load: Input,
    pub addr: Input16,

    pub out: Output16,
    // TODO: tty_ready?
}

impl Component for MemorySystem {
    type Target = Project05Component;

    fn expand(&self) -> Option<IC<Project05Component>> { todo!() }
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

    /// If true, the ALU is not involved; just load the bits of the instruction to the A register.
    pub load: Output,

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
    type Target = Project05Component;

    fn expand(&self) -> Option<IC<Project05Component>> {
        let mut components: Vec<Project05Component> = vec![];

        // NOT(NOT(src)) = src: a dumb way to express a plain wire using Nand gates.
        fn wrap(not: Not) -> Project05Component {
            let p01: Project01Component = not.into();
            let p02: Project02Component = p01.into();
            let p03: Project03Component = p02.into();
            p03.into()
        }
        let mut wire = |src: Input, dst: Output| {
            let mid  = Not { a: src, out: Not::chip().out };
            let pass = Not { a: mid.out.clone().into(), out: dst };
            for not in [mid, pass] { components.push(wrap(not)); }
        };

        wire(self.instr.bit(15).clone(), self.load.clone());
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

    pub mem_out: Output16,
    pub mem_write: Output,  // aka "load"
    pub mem_addr: Output16,

    pub mem_in: Input16,  // aka "data"
}

impl Component for CPU {
    type Target = Project05Component;

    fn expand(&self) -> Option<IC<Project05Component>> {  }
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

    fn expand(&self) -> Option<IC<Project05Component>> { todo!() }
}
