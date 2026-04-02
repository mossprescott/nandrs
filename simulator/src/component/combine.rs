//! Combinational primitives: `Nand` and `Buffer`, plus the `Combinational` enum that wraps them.

use frunk::Coproduct;

use crate::declare::BusRef;
use crate::{Chip, Component, IC, Input1, Interface, Output, Reflect};

/// The single primitive: true if either input is false.
#[derive(Clone, Reflect, Chip)]
pub struct Nand {
    pub a: Input1,
    pub b: Input1,
    pub out: Output,
}

/// "Gate" that just connects its input to its output without modifying it. For our purposes, this
/// is useful for connecting an input directly to an output in an IC.
///
/// Typically costs nothing at simulation time, and not counted as a gate when estimating chip
/// sizes.
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
#[derive(Clone, Reflect)]
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

/// Coproduct equivalent of `Combinational`; the eventual replacement.
///
/// Note: this explicit enumeration of types is what you use when you wnant to *consume*
/// a component of one of these types; you know exctly what types you know how to deal with
/// and need it to be one of them. Normally when you're producing a component, you want to
/// use a type constraint that just says which types need to be allowed.
pub type CombinationalT = frunk::Coprod!(Nand, Buffer);

impl From<Combinational> for CombinationalT {
    fn from(c: Combinational) -> Self {
        match c {
            Combinational::Nand(n) => Coproduct::inject(n),
            Combinational::Buffer(b) => Coproduct::inject(b),
        }
    }
}

impl From<CombinationalT> for Combinational {
    fn from(c: CombinationalT) -> Self {
        c.fold(frunk::hlist![Combinational::Nand, Combinational::Buffer])
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
