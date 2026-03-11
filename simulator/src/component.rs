use std::collections::HashMap;

use crate::{Component, IC, Input, InputBus, Output, OutputBus, Reflect, AsConst, Chip, Interface};
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
            inputs:  HashMap::from([
                ("a".to_string(),   self.a.clone().into()),
                ("b".to_string(),   self.b.clone().into()),
            ]),
            outputs: HashMap::from([
                ("out".to_string(), self.out.clone().into()),
            ]),
        }
    }
    fn name(&self) -> String { "Nand".into() }
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

/// No-cost "component" that just supplies some fixed zero/one bits.
///
/// Morally equivalent to a single-word ROM.
#[derive(Clone)]
pub struct Const {
    pub value: u64,

    // HACK: no particular reason this should be 16 bits.
    pub out: OutputBus<N16>,
}

impl Const {
    pub fn chip(value: u64) -> Self {
        Const { value, out: OutputBus::<N16>::new() }
    }
}

impl Reflect for Const {
    fn reflect(&self) -> Interface {
        Interface {
            inputs:  HashMap::new(),
            outputs: HashMap::from([
                ("out".to_string(), self.out.clone().into()),
            ]),
        }
    }
    fn name(&self) -> String { format!("Const({})", self.value) }
}

impl AsConst for Const {
    fn as_const(&self) -> Option<u64> { Some(self.value) }
}

/// Nothing to expand; Const is primitive.
impl Component for Const {
    type Target = Const;

    fn expand(&self) -> Option<IC<Const>> {
        None
    }
}

/// Type of components that participate in "combinational" circuits: only Nand and Const.
pub enum Combinational {
    Nand(Nand),
    Const(Const),
    // ROM?
}

impl From<Nand>  for Combinational { fn from(c: Nand)  -> Self { Combinational::Nand(c)  } }
impl From<Const> for Combinational { fn from(c: Const) -> Self { Combinational::Const(c) } }

impl Reflect for Combinational {
    fn reflect(&self) -> Interface {
        match self {
            Self::Nand(c)  => c.reflect(),
            Self::Const(c) => c.reflect(),
        }
    }
    fn name(&self) -> String {
        match self {
            Self::Nand(c)  => c.name(),
            Self::Const(c) => c.name(),
        }
    }
}

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
            inputs:  HashMap::from([
                ("data".to_string(), self.data.clone().into()),
                ("load".to_string(), self.load.clone().into()),
            ]),
            outputs: HashMap::from([
                ("out".to_string(), self.out.clone().into()),
            ]),
        }
    }
    fn name(&self) -> String { "Register".into() }
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

/// Type of components that participate in "sequential" circuits of a defined width: Combinational
/// and Register<Width>.
#[derive(Clone)]
pub enum Sequential<Width: Nat> {
    Nand(Nand),
    Const(Const),
    Register(Register<Width>),
}

impl<Width: Nat + Clone> Reflect for Sequential<Width> {
    fn reflect(&self) -> Interface {
        match self {
            Self::Nand(c)     => c.reflect(),
            Self::Const(c)    => c.reflect(),
            Self::Register(c) => c.reflect(),
        }
    }
    fn name(&self) -> String {
        match self {
            Self::Nand(c)     => c.name(),
            Self::Const(c)    => c.name(),
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
pub struct RAM<A: Nat, D: Nat> {
    /// Capacity of the RAM in words; <= 2^address_bits. Valid addresses are 0 to size-1.
    pub size: usize,

    pub addr: InputBus<A>,

    pub data: InputBus<D>,
    pub load: Input,

    pub out: OutputBus<D>,
}

impl<A: Nat + Clone, D: Nat + Clone> Reflect for RAM<A, D> {
    fn reflect(&self) -> Interface {
        Interface {
            inputs: HashMap::from([
                ("addr".to_string(), self.addr.clone().into()),
                ("data".to_string(), self.data.clone().into()),
                ("load".to_string(), self.load.clone().into()),
            ]),
            outputs: HashMap::from([
                ("out".to_string(), self.out.clone().into()),
            ]),
        }
    }
    fn name(&self) -> String { "RAM".into() }
}

impl<A: Nat, D: Nat> RAM<A, D> {
    pub fn chip(size: usize) -> Self {
        RAM { size: size, addr: InputBus::new(), data: InputBus::new(), load: Input::new(), out: OutputBus::<D>::new() }
    }
}

/// Nothing to expand; RAM is primitive for the simulator.
impl<A: Nat, D: Nat> Component for RAM<A, D> {
    type Target = RAM<A, D>;

    fn expand(&self) -> Option<IC<RAM<A, D>>> {
        None
    }
}

#[derive(Clone)]
pub struct ROM<A: Nat, D: Nat> {
    /// Capacity of the ROM in words; <= 2^address_bits. Valid addresses are 0 to size-1.
    pub size: usize,

    pub addr: InputBus<A>,

    pub out: OutputBus<D>,
}

impl<A: Nat + Clone, D: Nat + Clone> Reflect for ROM<A, D> {
    fn reflect(&self) -> Interface {
        Interface {
            inputs: HashMap::from([
                ("addr".to_string(), self.addr.clone().into()),
            ]),
            outputs: HashMap::from([
                ("out".to_string(), self.out.clone().into()),
            ]),
        }
    }
    fn name(&self) -> String { "ROM".into() }
}

impl<A: Nat, D: Nat> ROM<A, D> {
    pub fn chip(size: usize) -> Self {
        ROM { size: size, addr: InputBus::<A>::new(), out: OutputBus::<D>::new() }
    }
}

/// Nothing to expand; ROM is primitive for the simulator.
impl<A: Nat, D: Nat> Component for ROM<A, D> {
    type Target = ROM<A, D>;

    fn expand(&self) -> Option<IC<ROM<A, D>>> {
        None
    }
}


// - Computational

/// Type of components that participate in computers, including logic, registers, memory, and I/O.
#[derive(Clone)]
pub enum Computational<A: Nat, D: Nat> {
    Nand(Nand),
    Const(Const),
    Register(Register<D>),
    /// Note: typically not all of the address bits are used, but also multiple RAMs with
    /// different address widths would be most precise and that's just not worth it for now.
    RAM(RAM<A, D>),
    ROM(ROM<A, D>),
    // TODO: I/O (Keyboard, TTY)
}

impl<A: Nat + Clone, D: Nat + Clone> Reflect for Computational<A, D> {
    fn reflect(&self) -> Interface {
        match self {
            Self::Nand(c)     => c.reflect(),
            Self::Const(c)    => c.reflect(),
            Self::Register(c) => c.reflect(),
            Self::RAM(c)      => c.reflect(),
            Self::ROM(c)      => c.reflect(),
        }
    }
    fn name(&self) -> String {
        match self {
            Self::Nand(c)     => c.name(),
            Self::Const(c)    => c.name(),
            Self::Register(c) => c.name(),
            Self::RAM(c)      => c.name(),
            Self::ROM(c)      => c.name(),
        }
    }
}

impl<A: Nat, D: Nat> Component for Computational<A, D> {
    type Target = Self;

    fn expand(&self) -> Option<IC<Self::Target>> {
        None
    }
}

pub type RAM16           = RAM<N16, N16>;
pub type ROM16           = ROM<N16, N16>;
pub type Computational16 = Computational<N16, N16>;

impl<A: Nat, D: Nat> From<Sequential<D>> for Computational<A, D> {
    fn from(s: Sequential<D>) -> Self {
        match s {
            Sequential::Nand(n)     => Computational::Nand(n),
            Sequential::Const(n)    => Computational::Const(n),
            Sequential::Register(r) => Computational::Register(r),
        }
    }
}
