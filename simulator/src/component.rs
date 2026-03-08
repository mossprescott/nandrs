use crate::{Component, IC, Input, InputBus, Output, OutputBus, Reflect, Chip, Interface};
use crate::nat::{Nat, N16};

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

    fn expand(&self) -> Option<IC<Nand>> {
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

    fn expand(&self) -> Option<IC<Register<Width>>> {
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

impl<Width: Nat + Clone> Reflect for Sequential<Width> {
    fn reflect(&self) -> Interface {
        match self {
            Self::Nand(c) => c.reflect(),
            Self::Register(c) => c.reflect(),
        }
    }
    fn name(&self) -> &str {
        match self {
            Self::Nand(c) => c.name(),
            Self::Register(c) => c.name(),
        }
    }
}

impl<Width: Nat> Component for Sequential<Width> {
    type Target = Self;

    fn expand(&self) -> Option<IC<Self::Target>> {
        None
    }
}

pub type Sequential16 = Sequential<N16>;

// TODO: "Computer?" sequential, plus RAM, ROM, Keyboard, and TTY
// Except... Generalize Keyboard and TTY to some kind of I/O device provided by the harness.
// It could be keyboard and debug trace, or it could be the host terminal, etc.
