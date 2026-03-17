#![allow(unused_variables, dead_code, unused_imports)]

use simulator::{self, Component, IC, Input, Input16, Output, Output16, Reflect, AsConst, Chip};
use simulator::Reflect as _;
use simulator::Chip as _;
use simulator::component::{Combinational, Const, Nand, Register16, Sequential, Sequential16};
use crate::project_01::{Or, Mux16, Project01Component};
use crate::project_02::{Inc16, Project02Component};

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
    fn name(&self) -> String {
        match self {
            Project03Component::Project02(c) => c.name(),
            Project03Component::Register16(c) => c.name(),
            Project03Component::PC(c) => c.name(),
        }
    }
}

impl AsConst for Project03Component {
    fn as_const(&self) -> Option<u64> {
        if let Project03Component::Project02(c) = self { c.as_const() } else { None }
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
                        .map(|c| match c {
                            Combinational::Nand(n)   => Sequential::Nand(n),
                            Combinational::Const(c)  => Sequential::Const(c),
                            Combinational::Buffer(c) => Sequential::Buffer(c),
                            Combinational::Mux(m)    => Sequential::Mux(m),
                            Combinational::Mux1(m)   => Sequential::Mux1(m),
                            Combinational::Adder(a)  => Sequential::Adder(a),
                        })
                        .collect(),
                Project03Component::Register16(reg) => vec![Sequential::Register(reg)],
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

/// Program counter component, including a register storing the current instruction address.
///
/// When more than one flag is set, "reset" supercedes "load", which supercedes "inc".
#[derive(Reflect, Chip)]
pub struct PC {
    /// Reset to zero on the next cycle
    pub reset: Input,

    /// Load an arbitrary address
    pub addr: Input16,
    pub load: Input,

    /// Increment to point to the next address on the next cycle
    pub inc: Input,

    pub out: Output16,
}

impl Component for PC {
    type Target = Project03Component;

    // Note: no special ceremony needed for back-references to the register's output, because
    // that wire is already declared as the output "out".
    fn expand(&self) -> Option<IC<Project03Component>> {
        let zero = Const { value: 0, out: Output16::new() };

        let inc = Inc16 { a: self.out.clone().into(), out: Output16::new() };
        let next0 = Mux16 { a0: self.out.clone().into(), a1: inc.out.clone().into(), sel: self.inc.clone(), out: Output16::new() };

        let next1 = Mux16 { a0: next0.out.clone().into(), a1: self.addr.clone(), sel: self.load.clone(), out: Output16::new() };

        let next2 = Mux16 { a0: next1.out.clone().into(), a1: zero.out.clone().into(), sel: self.reset.clone(), out: Output16::new() };

        let any0 = Or { a: self.inc.clone(), b: self.load.clone(), out: Output::new() };
        let any = Or { a: any0.out.clone().into(), b: self.reset.clone(), out: Output::new() };

        let reg = Register16 {
            data_in:  next2.out.clone().into(),
            write:    any.out.clone().into(),
            data_out: self.out.clone(),
        };

        Some(IC { name: self.name().to_string(), intf: self.reflect(), components: vec![
            // FIXME: horrific
            Project02Component::from(inc).into(),
            Project02Component::from(Project01Component::from(next0)).into(),
            Project02Component::from(Project01Component::from(next1)).into(),
            Project02Component::from(Project01Component::from(next2)).into(),
            Project02Component::from(Project01Component::from(any0)).into(),
            Project02Component::from(Project01Component::from(any)).into(),
            reg.into(),
        ]})
    }
}
