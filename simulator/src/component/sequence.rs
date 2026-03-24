//! Sequential primitives: `Register`, plus the `Sequential` enum combining registers with
//! combinational components.

use crate::declare::BusRef;
use crate::nat::{N16, Nat};
use crate::{Chip, Component, IC, Input, Input1, Interface, OutputBus, Reflect};

use super::{Buffer, Nand};

#[derive(Clone, Reflect, Chip)]
pub struct Register<Width: Nat> {
    pub data_in: Input<Width>,
    pub write: Input1,
    pub data_out: OutputBus<Width>,
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
/// and `Register<Width>`.
#[derive(Clone, Reflect)]
pub enum Sequential<Width: Nat> {
    Nand(Nand),
    Buffer(Buffer),
    Register(Register<Width>),
}

impl<Width: Nat> From<Nand> for Sequential<Width> {
    fn from(c: Nand) -> Self {
        Sequential::Nand(c)
    }
}
impl<Width: Nat> From<Buffer> for Sequential<Width> {
    fn from(c: Buffer) -> Self {
        Sequential::Buffer(c)
    }
}
impl<Width: Nat> From<Register<Width>> for Sequential<Width> {
    fn from(c: Register<Width>) -> Self {
        Sequential::Register(c)
    }
}

impl<Width: Nat> Component for Sequential<Width> {
    type Target = Self;

    fn expand(&self) -> Option<IC<Self::Target>> {
        None
    }
}

pub type Sequential16 = Sequential<N16>;

pub struct SequentialCounts {
    pub nands: usize,
    pub buffers: usize,
    pub registers: usize,
}

pub fn count_sequential<W: Nat>(components: &[Sequential<W>]) -> SequentialCounts {
    let mut counts = SequentialCounts {
        nands: 0,
        buffers: 0,
        registers: 0,
    };
    for comp in components {
        match comp {
            Sequential::Nand(_) => counts.nands += 1,
            Sequential::Buffer(_) => counts.buffers += 1,
            Sequential::Register(_) => counts.registers += 1,
        }
    }
    counts
}
