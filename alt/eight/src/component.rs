use assignments::project_05::Decode;
use simulator::component::{MemorySystem16, ROM16, Register, WiredRegister};
use simulator::declare::{BusRef, Interface};
use simulator::nat::{N1, N8};
use simulator::{Chip, Input, Input1, Output, OutputBus, Reflect, fixed};

// Re-export for use in computer.rs
pub use assignments::project_05::Decode as _Decode;
pub use simulator::component::{MemorySystem16 as _MemorySystem16, ROM16 as _ROM16};

pub type Input8 = Input<N8>;
pub type Output8 = OutputBus<N8>;

/// Wrap an 8-bit register as a (trivial) component with its own distinct type, because Rust's
/// cross-crate trait resolution is happier that way.
#[derive(Clone, Reflect, Chip)]
pub struct Register8 {
    pub data_in: Input8,
    pub write: Input1,
    pub data_out: Output8,
}

impl From<Register8> for WiredRegister {
    fn from(r: Register8) -> Self {
        Register {
            data_in: r.data_in,
            write: r.write,
            data_out: r.data_out,
        }
        .into()
    }
}

/// An 8-bit latch (register with write always 1). Component impl is in computer.rs.
#[derive(Clone, Reflect, Chip)]
pub struct Latch8 {
    pub data_in: Input8,
    pub data_out: Output8,
}

/// Wrap a single-bit latch as a (trivial) component with its own distinct type, because Rust's
/// cross-crate trait resolution is happier that way.
#[derive(Clone, Reflect, Chip)]
pub struct Latch1 {
    pub data_in: Input1,
    pub data_out: Output,
}

impl From<Latch1> for WiredRegister {
    fn from(r: Latch1) -> Self {
        WiredRegister {
            width: 1,
            data_in: BusRef::from_input(r.data_in),
            write: BusRef::from_input(fixed::<N1>(1)),
            data_out: BusRef::from_output(r.data_out),
        }
    }
}

/// Local newtype for Decode, so `From<EightDecode> for Combinational8` is coherent.
/// Component impl (which references Combinational8) is in computer.rs.
#[derive(Clone)]
pub struct EightDecode(pub Decode);

impl Reflect for EightDecode {
    fn name(&self) -> String {
        self.0.name()
    }
    fn reflect(&self) -> Interface {
        self.0.reflect()
    }
}
impl Chip for EightDecode {
    fn chip() -> Self {
        EightDecode(Decode::chip())
    }
}
impl std::ops::Deref for EightDecode {
    type Target = Decode;
    fn deref(&self) -> &Decode {
        &self.0
    }
}
impl std::ops::DerefMut for EightDecode {
    fn deref_mut(&mut self) -> &mut Decode {
        &mut self.0
    }
}

/// Local newtype for ROM16, so `From<EightROM> for EightComponent` is coherent.
#[derive(Clone)]
pub struct EightROM(pub ROM16);

impl Reflect for EightROM {
    fn name(&self) -> String {
        self.0.name()
    }
    fn reflect(&self) -> Interface {
        self.0.reflect()
    }
}
impl Chip for EightROM {
    fn chip() -> Self {
        EightROM(ROM16::chip(0))
    }
}
impl std::ops::Deref for EightROM {
    type Target = ROM16;
    fn deref(&self) -> &ROM16 {
        &self.0
    }
}
impl std::ops::DerefMut for EightROM {
    fn deref_mut(&mut self) -> &mut ROM16 {
        &mut self.0
    }
}

/// Local newtype for MemorySystem16, so `From<EightMemSys> for EightComponent` is coherent.
#[derive(Clone)]
pub struct EightMemSys(pub MemorySystem16);

impl Reflect for EightMemSys {
    fn name(&self) -> String {
        self.0.name()
    }
    fn reflect(&self) -> Interface {
        self.0.reflect()
    }
}
impl Chip for EightMemSys {
    fn chip() -> Self {
        EightMemSys(MemorySystem16::chip())
    }
}
impl std::ops::Deref for EightMemSys {
    type Target = MemorySystem16;
    fn deref(&self) -> &MemorySystem16 {
        &self.0
    }
}
impl std::ops::DerefMut for EightMemSys {
    fn deref_mut(&mut self) -> &mut MemorySystem16 {
        &mut self.0
    }
}
