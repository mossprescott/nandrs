//! Sequential primitives: `Register`, plus the `Sequential` enum combining registers with
//! combinational components.

// use std::collections::HashMap;

use crate::declare::BusRef;
use crate::{Chip, Input1, Interface, Output, Reflect};

use super::{Buffer, Nand};

/// Primitive memory cell, storing a single bit across each clock cycle.
#[derive(Clone, Reflect, Chip)]
pub struct DFF {
    pub a: Input1,
    pub out: Output,
}

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
