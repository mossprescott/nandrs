#![allow(unused_variables, dead_code, unused_imports)]

use simulator::{self, Component, IC, Input, Input16, Output, Output16, Reflect, Chip};
use simulator::Reflect as _;
use simulator::Chip as _;
use simulator::component::{Nand, Register16, Sequential, Sequential16};
use crate::project_02::Project02Component;

pub enum Project03Component {
    Project02(Project02Component),
    Register16(Register16),
    PC(PC),
}
impl From<Project02Component> for Project03Component { fn from(c: Project02Component) -> Self { Project03Component::Project02(c) } }
impl From<Register16> for Project03Component { fn from(c: Register16) -> Self { Project03Component::Register16(c) } }
impl From<PC> for Project03Component { fn from(c: PC) -> Self { Project03Component::PC(c) } }

impl Component for Project03Component {
    type Target = Project03Component;

    fn expand(&self) -> Option<IC<Project03Component>> {
        match self {
            Project03Component::Project02(c) => c.expand().map(|ic| IC { name: ic.name, intf: ic.intf, components: ic.components.into_iter().map(Into::into).collect() }),
            Project03Component::Register16(c) => c.expand().map(|ic| unreachable!()),
            Project03Component::PC(c) => c.expand(),
        }
    }
}

impl Reflect for Project03Component {
    fn reflect(&self) -> simulator::Interface {
        match self {
            Project03Component::Project02(c) => c.reflect(),
            Project03Component::Register16(c) => c.reflect(),
            Project03Component::PC(c) => c.reflect(),
        }
    }
    fn name(&self) -> &str {
        match self {
            Project03Component::Project02(c) => c.name(),
            Project03Component::Register16(c) => c.name(),
            Project03Component::PC(c) => c.name(),
        }
    }
}


/// Recursively expand until only Nands and Registers are left.
pub fn flatten<C: Reflect + Into<Project03Component>>(chip: C) -> IC<Sequential16> {
    fn go(comp: Project03Component) -> Vec<Sequential16> {
        match comp.expand() {
            None => match comp {
                Project03Component::Project02(p) =>
                    crate::project_02::flatten(p)
                        .components.into_iter()
                        .map(|nand| Sequential::Nand(nand))
                        .collect(),
                _ => panic!("Did not reduce to Nand/Register: {:?}", comp.name()),
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
pub struct PC {
    /// Load an arbitrary address
    pub addr: Input16,
    pub load: Input,

    /// Increment to point to the next address on the next cycle
    pub inc: Input,

    /// Reset to zero on the next cycle
    pub reset: Input,

    pub out: Output16,
}

impl Component for PC {
    type Target = Project03Component;

    fn expand(&self) -> Option<IC<Project03Component>> {
        todo!()
    }
}
