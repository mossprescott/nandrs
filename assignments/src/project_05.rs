#![allow(unused_variables, dead_code, unused_imports)]

use simulator::{self, Component, IC, Input, Input16, Output, Output16, Reflect, Chip};
use simulator::Reflect as _;
use simulator::Chip as _;
use simulator::component::{Nand, Register16, RAM16, ROM16, Sequential, Computational, Computational16};
use crate::project_03::Project03Component;

pub enum Project05Component {
    Project03(Project03Component),
    RAM(RAM16),
    ROM(ROM16),
    MemorySystem(MemorySystem),
    CPU(CPU),
    Computer(Computer),
}

impl From<Project03Component> for Project05Component { fn from(c: Project03Component) -> Self { Project05Component::Project03(c) } }
impl From<RAM16>              for Project05Component { fn from(c: RAM16)              -> Self { Project05Component::RAM(c)         } }
impl From<ROM16>              for Project05Component { fn from(c: ROM16)              -> Self { Project05Component::ROM(c)         } }
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

    fn expand(&self) -> Option<IC<Project05Component>> { todo!() }
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
