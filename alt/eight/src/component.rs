use simulator::component::{Register, WiredRegister};
use simulator::declare::{BusRef, Interface};
use simulator::nat::{N1, N8};
use simulator::{Chip, Input, Input1, Output, OutputBus, Reflect, expand_t, fixed};

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

/// An 8-bit latch (register with write always 1).
#[derive(Clone, Reflect, Chip)]
pub struct Latch8 {
    pub data_in: Input8,
    pub data_out: Output8,
}
impl Latch8 {
    expand_t!([Register8], |this| {
        reg: Register8 { data_in: this.data_in, write: fixed(1), data_out: this.data_out },
    });
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
