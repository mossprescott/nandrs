//! Sequential primitives: `Register`, plus the `Sequential` enum combining registers with
//! combinational components.

use std::collections::HashMap;

use crate::declare::BusRef;
use crate::nat::{N16, Nat};
use crate::{Chip, Input, Input1, Interface, OutputBus, Reflect};

use super::{Buffer, Nand};

/// Primitive memory cell, storing one or more bits often treated as a binary number.
#[derive(Clone, Reflect, Chip)]
pub struct Register<Width: Nat> {
    pub data_in: Input<Width>,
    pub write: Input1,
    pub data_out: OutputBus<Width>,
}

pub type Register16 = Register<N16>;

/// Records the wiring of a Register, including the width in bits as a runtime value.
#[derive(Clone)]
pub struct WiredRegister {
    pub width: usize,

    pub data_in: BusRef,
    pub write: BusRef,
    pub data_out: BusRef,
}

impl Reflect for WiredRegister {
    fn name(&self) -> String {
        "Register".to_string()
    }

    fn reflect(&self) -> Interface {
        Interface {
            inputs: HashMap::from([
                ("data_in".to_string(), self.data_in),
                ("write".to_string(), self.write),
            ]),
            outputs: HashMap::from([("data_out".to_string(), self.data_out)]),
        }
    }
}

impl<Width: Nat> From<Register<Width>> for WiredRegister {
    fn from(c: Register<Width>) -> Self {
        WiredRegister {
            width: Width::as_int(),
            data_in: BusRef::from_input(c.data_in),
            write: BusRef::from_input(c.write),
            data_out: BusRef::from_output(c.data_out),
        }
    }
}

/// Type of components that participate in "sequential" circuits: `Combinational` and `Register`.
///
/// Note: a single chip can contain registers (and other components) that have various bit-widths,
/// so there's no single "width" parameter at type- or value-level..
#[derive(Clone, Reflect)]
pub enum Sequential {
    Nand(Nand),
    Buffer(Buffer),
    Register(WiredRegister),
}

impl From<Nand> for Sequential {
    fn from(c: Nand) -> Self {
        Sequential::Nand(c)
    }
}
impl From<Buffer> for Sequential {
    fn from(c: Buffer) -> Self {
        Sequential::Buffer(c)
    }
}
impl<Width: Nat> From<Register<Width>> for Sequential {
    fn from(c: Register<Width>) -> Self {
        Sequential::Register(c.into())
    }
}

pub struct SequentialCounts {
    pub nands: usize,
    pub buffers: usize,
    pub registers: usize,
}

pub fn count_sequential(components: &[Sequential]) -> SequentialCounts {
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
