//! Combinational primitives: `Nand` and `Buffer`, plus the `Combinational` enum that wraps them.

use crate::declare::BusRef;
use crate::{Chip, Component, IC, Input1, Interface, Output, Reflect};

/// The single primitive: true if either input is false.
#[derive(Clone, Reflect, Chip)]
pub struct Nand {
    pub a: Input1,
    pub b: Input1,
    pub out: Output,
}

/// "Gate" that just connects its input to its output without modifying it. For our purposes,
/// this is useful for connecting an input directly to an output in an IC.
#[derive(Clone, Reflect)]
pub struct Buffer {
    pub a: Input1,
    pub out: Output,
}

/// Nothing to expand; Buffer is primitive.
impl Component for Buffer {
    type Target = Buffer;

    fn expand(&self) -> Option<IC<Buffer>> {
        None
    }
}

/// Type of components that participate in "combinational" circuits:
/// - most importantly `Nand`
/// - `Buffer` for pass-through connections
pub enum Combinational {
    Nand(Nand),
    Buffer(Buffer),
}

impl From<Nand> for Combinational {
    fn from(c: Nand) -> Self {
        Combinational::Nand(c)
    }
}
impl From<Buffer> for Combinational {
    fn from(c: Buffer) -> Self {
        Combinational::Buffer(c)
    }
}

impl Reflect for Combinational {
    fn reflect(&self) -> Interface {
        match self {
            Self::Nand(c) => c.reflect(),
            Self::Buffer(c) => c.reflect(),
        }
    }
    fn name(&self) -> String {
        match self {
            Self::Nand(c) => c.name(),
            Self::Buffer(c) => c.name(),
        }
    }
}

pub struct CombinationalCounts {
    pub nands: usize,
    pub buffers: usize,
}

pub fn count_combinational(components: &[Combinational]) -> CombinationalCounts {
    let mut counts = CombinationalCounts {
        nands: 0,
        buffers: 0,
    };
    for comp in components {
        match comp {
            Combinational::Nand(_) => counts.nands += 1,
            Combinational::Buffer(_) => counts.buffers += 1,
        }
    }
    counts
}
