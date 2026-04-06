//! Sequential primitives: `Register`, plus the `Sequential` enum combining registers with
//! combinational components.

// use std::collections::HashMap;

use crate::declare::BusRef;
use crate::{Chip, Input1, Interface, Output, Reflect};

use super::{Buffer, Nand};

/// Primitive memory cell, storing
#[derive(Clone, Reflect, Chip)]
pub struct DFF {
    pub a: Input1,
    pub out: Output,
}

// pub type Register16 = Register<N16>;

// /// Records the wiring of a Register, including the width in bits as a runtime value.
// ///
// /// This allows registers of different widths to be included in the same circuit as components get
// /// expanded. Or something like that.
// #[derive(Clone)]
// pub struct WiredRegister {
//     pub width: usize,

//     pub data_in: BusRef,
//     pub write: BusRef,
//     pub data_out: BusRef,
// }

// impl Reflect for WiredRegister {
//     fn name(&self) -> String {
//         "Register".to_string()
//     }

//     fn reflect(&self) -> Interface {
//         Interface {
//             inputs: HashMap::from([
//                 ("data_in".to_string(), self.data_in),
//                 ("write".to_string(), self.write),
//             ]),
//             outputs: HashMap::from([("data_out".to_string(), self.data_out)]),
//         }
//     }
// }

// impl<Width: Nat> From<Register<Width>> for WiredRegister {
//     fn from(c: Register<Width>) -> Self {
//         WiredRegister {
//             width: Width::as_int(),
//             data_in: BusRef::from_input(c.data_in),
//             write: BusRef::from_input(c.write),
//             data_out: BusRef::from_output(c.data_out),
//         }
//     }
// }

/// Type of components that participate in "sequential" circuits: `Combinational` and `Register`.
///
/// Note: a single chip can contain registers (and other components) that have various bit-widths,
/// so there's no single "width" parameter at type- or value-level..
#[derive(Clone, Reflect)]
pub enum Sequential {
    Nand(Nand),
    Buffer(Buffer),
    DFF(DFF),
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
impl From<DFF> for Sequential {
    fn from(c: DFF) -> Self {
        Sequential::DFF(c)
    }
}
// impl<Width: Nat> From<Register<Width>> for Sequential {
//     fn from(c: Register<Width>) -> Self {
//         Sequential::Register(c.into())
//     }
// }

pub struct SequentialCounts {
    pub nands: usize,
    pub buffers: usize,
    pub dffs: usize,
}

pub fn count_sequential(components: &[Sequential]) -> SequentialCounts {
    let mut counts = SequentialCounts {
        nands: 0,
        buffers: 0,
        dffs: 0,
    };
    for comp in components {
        match comp {
            Sequential::Nand(_) => counts.nands += 1,
            Sequential::Buffer(_) => counts.buffers += 1,
            Sequential::DFF(_) => counts.dffs += 1,
        }
    }
    counts
}
