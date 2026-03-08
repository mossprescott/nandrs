use crate::{Component, IC, Input, InputBus, Output, OutputBus, Reflect, Chip, Interface};
use crate::nat::{Nat, N16};


// - Nand (Combinational)

/// The single primitive: true if either input is false.
#[derive(Clone)]
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

/// Type of components that participate in "cobinational" circuits: only Nand.
pub type Combinational = Nand;


// - Registers (Sequential)

#[derive(Clone)]
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

/// Type of components that participate in "sequential" circuits of a defined width: only Nand
/// and Register<Width>.
#[derive(Clone)]
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


// - Memory and I/O (Computational)

#[derive(Clone)]
pub struct RAM<Width: Nat> {
    pub addr: InputBus<Width>,

    pub data: InputBus<Width>,
    pub load: Input,

    pub out: OutputBus<Width>,
}

impl<Width: Nat + Clone> Reflect for RAM<Width> {
    fn reflect(&self) -> Interface {
        Interface {
            inputs: std::collections::HashMap::from([
                ("addr".to_string(), self.addr.clone().into()),
                ("data".to_string(), self.data.clone().into()),
                ("load".to_string(), self.load.clone().into()),
            ]),
            outputs: std::collections::HashMap::from([
                ("out".to_string(), self.out.clone().into()),
            ]),
        }
    }
    fn name(&self) -> &str { "RAM" }
}

impl<Width: Nat> Chip for RAM<Width> {
    fn chip() -> Self {
        RAM { addr: InputBus::new(), data: InputBus::new(), load: Input::new(), out: OutputBus::<Width>::new() }
    }
}

/// Nothing to expand; RAM is primitive for the simulator.
impl<Width: Nat> Component for RAM<Width> {
    type Target = RAM<Width>;

    fn expand(&self) -> Option<IC<RAM<Width>>> {
        None
    }
}

#[derive(Clone)]
pub struct ROM<Width: Nat> {
    pub addr: InputBus<Width>,

    pub out: OutputBus<Width>,
}

impl<Width: Nat + Clone> Reflect for ROM<Width> {
    fn reflect(&self) -> Interface {
        Interface {
            inputs: std::collections::HashMap::from([
                ("addr".to_string(), self.addr.clone().into()),
            ]),
            outputs: std::collections::HashMap::from([
                ("out".to_string(), self.out.clone().into()),
            ]),
        }
    }
    fn name(&self) -> &str { "ROM" }
}

impl<Width: Nat> Chip for ROM<Width> {
    fn chip() -> Self {
        ROM { addr: InputBus::<Width>::new(), out: OutputBus::<Width>::new() }
    }
}

/// Nothing to expand; ROM is primitive for the simulator.
impl<Width: Nat> Component for ROM<Width> {
    type Target = ROM<Width>;

    fn expand(&self) -> Option<IC<ROM<Width>>> {
        None
    }
}

/// Type of components that participate in computers, including logic, registers, memory, and I/O.
#[derive(Clone)]
pub enum Computational<Width: Nat> {
    Nand(Nand),
    Register(Register<Width>),
    RAM(RAM<Width>),
    ROM(ROM<Width>),
    // TODO: I/O (Keyboard, TTY)
}

impl<Width: Nat + Clone> Reflect for Computational<Width> {
    fn reflect(&self) -> Interface {
        match self {
            Self::Nand(c)     => c.reflect(),
            Self::Register(c) => c.reflect(),
            Self::RAM(c)      => c.reflect(),
            Self::ROM(c)      => c.reflect(),
        }
    }
    fn name(&self) -> &str {
        match self {
            Self::Nand(c)     => c.name(),
            Self::Register(c) => c.name(),
            Self::RAM(c)      => c.name(),
            Self::ROM(c)      => c.name(),
        }
    }
}

impl<Width: Nat> Component for Computational<Width> {
    type Target = Self;

    fn expand(&self) -> Option<IC<Self::Target>> {
        None
    }
}

pub type RAM16 = RAM<N16>;
pub type ROM16 = ROM<N16>;
pub type Computational16 = Computational<N16>;

impl<Width: Nat> From<Sequential<Width>> for Computational<Width> {
    fn from(s: Sequential<Width>) -> Self {
        match s {
            Sequential::Nand(n)     => Computational::Nand(n),
            Sequential::Register(r) => Computational::Register(r),
        }
    }
}
