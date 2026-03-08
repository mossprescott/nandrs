use crate::{Component, Input, InputBus, Output, OutputBus, Reflect, Chip, Interface};
use crate::nat::{Nat, N16};

/// A circuit composed of inputs, outputs, and zero or more components of a certain type.
///
/// Invariant: every input of every component must refer to either: one of the inputs of
/// self.intf, or an output associated with some other component in the same IC.
pub struct IC<C> {
    pub name: String,

    /// The exposed inputs and outputs.
    pub intf: Interface,

    /// The constituent components.
    pub components: Vec<C>,
}
impl<C> Reflect for IC<C> {
    fn reflect(&self) -> Interface {
        self.intf.clone()
    }

    fn name(&self) -> &str {
        &self.name
    }
}

/// The single primitive: true if either input is false.
pub struct Nand {
    pub a: Input,
    pub b: Input,
    pub out: Output,
}

impl Reflect for Nand {
    fn reflect(&self) -> Interface {
        Interface {
            inputs:  std::collections::HashMap::from([
                ("a".to_string(),   self.a.clone().into()),
                ("b".to_string(),   self.b.clone().into()),
            ]),
            outputs: std::collections::HashMap::from([
                ("out".to_string(), self.out.clone().into()),
            ]),
        }
    }
    fn name(&self) -> &str { "Nand" }
}

impl Chip for Nand {
    fn chip() -> Self {
        Nand { a: Input::new(), b: Input::new(), out: Output::new() }
    }
}

/// Nothing to expand; Nand is Nand.
impl Component for Nand {
    type Target = Nand;

    fn expand(&self) -> Option<Vec<Nand>> {
        None
    }
}

pub struct Register<Width: Nat> {
    pub data: InputBus<Width>,
    pub load: Input,
    pub out: OutputBus<Width>,
}

impl<Width: Nat + Clone> Reflect for Register<Width> {
    fn reflect(&self) -> Interface {
        Interface {
            inputs:  std::collections::HashMap::from([
                ("data".to_string(), self.data.clone().into()),
                ("load".to_string(), self.load.clone().into()),
            ]),
            outputs: std::collections::HashMap::from([
                ("out".to_string(), self.out.clone().into()),
            ]),
        }
    }
    fn name(&self) -> &str { "Register" }
}

impl<Width: Nat> Chip for Register<Width> {
    fn chip() -> Self {
        Register { data: InputBus::new(), load: Input::new(), out: OutputBus::<Width>::new() }
    }
}

/// Nothing to expand; Register is primitive for the simulator we envisage.
impl<Width: Nat> Component for Register<Width> {
    type Target = Register<Width>;

    fn expand(&self) -> Option<Vec<Register<Width>>> {
        None
    }
}

pub type Register16 = Register<N16>;


/// Type of components that participate in "cobinational" circuits: only Nand.
pub type Combinational = Nand;

/// Type of components that participate in "sequential" circuits of a defined width: only Nand
/// and Register<Width>.
pub enum Sequential<Width: Nat> {
    Nand(Nand),
    Register(Register<Width>),
}

// TODO: "Computer?" sequential, plus RAM, ROM, Keyboard, and TTY
// Except... Generalize Keyboard and TTY to some kind of I/O device provided by the harness.
// It could be keyboard and debug trace, or it could be the host terminal, etc.
