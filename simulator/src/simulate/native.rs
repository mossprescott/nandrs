/// Components that aren't strictly primitive (or don't need to be), but which are provided as
/// "native" in that the simulator implements them directly for performance reasons.

use crate::{Component, IC, Input, Input1, Output, OutputBus, Reflect, Chip, Interface};
use crate::component::Computational;
use crate::declare::BusRef;
use crate::nat::{Nat, N1, N16, IsGreater};

/// Mux: out = if sel { a1 } else { a0 }, applied bitwise across Width bits.
#[derive(Clone, Reflect, Chip)]
pub struct Mux<Width: Nat> {
    pub a0: Input<Width>,
    pub a1: Input<Width>,
    pub sel: Input1,
    pub out: OutputBus<Width>,
}

/// Nothing to expand.
impl<Width: Nat> Component for Mux<Width> {
    type Target = Mux<Width>;

    fn expand(&self) -> Option<IC<Mux<Width>>> {
        None
    }
}

pub type Mux1 = Mux<N1>;
pub type Mux16 = Mux<N16>;

/// Single-bit slice off a multi-bit adder: adds three bits, producing a two-bit result.
///
/// Note that this primitive has the same interface and behavior as the project_02::FullAdder, which
/// reduces to only Nands. In fact, we require that to be the case, so that we can use one or the
/// other implementation for different purposes: this one, for efficient simulation; the other, for
/// counting the number of gates in a "realistic" implementation.
///
/// sum = 1s-digit of three-bit sum, carry = 2s-digit
#[derive(Clone, Reflect, Chip)]
pub struct Adder {
    /// "Left" input bit:
    pub a: Input1,
    /// "Right" input bit:
    pub b: Input1,
    /// "Carry-in" bit:
    pub c: Input1,

    /// 1s digit of a + b + c:
    pub sum: Output,
    /// 2s digit of a + b + c:
    pub carry: Output,
}

/// Nothing to expand.
impl Component for Adder {
    type Target = Adder;

    fn expand(&self) -> Option<IC<Adder>> { None }
}

/// The type of components that participate in computers for simulation purposes: this includes the
/// native components here in addition to the actual primitives of the Computational type.
pub enum Simulational<A: Nat, D: Nat> {
    Primitive(Computational<A, D>),
    Mux(Mux<D>),
    Mux1(Mux<N1>),
    Adder(Adder),
}

impl<A: Nat + Clone, D: Nat + Clone> Reflect for Simulational<A, D> {
    fn reflect(&self) -> Interface {
        match self {
            Self::Primitive(c) => c.reflect(),
            Self::Mux(c)      => c.reflect(),
            Self::Mux1(c)     => c.reflect(),
            Self::Adder(c)    => c.reflect(),
        }
    }
    fn name(&self) -> String {
        match self {
            Self::Primitive(c) => c.name(),
            Self::Mux(c)      => c.name(),
            Self::Mux1(c)     => c.name(),
            Self::Adder(c)    => c.name(),
        }
    }
}

impl<A: Nat, D: Nat> From<Computational<A, D>> for Simulational<A, D> {
    fn from(c: Computational<A, D>) -> Self { Simulational::Primitive(c) }
}

impl<A: Nat, D: Nat> From<Mux<D>> for Simulational<A, D>
  where D: IsGreater<N1>
{
    fn from(c: Mux<D>) -> Self { Simulational::Mux(c) }
}

impl<A: Nat, D: Nat> From<Mux<N1>> for Simulational<A, D> {
    fn from(c: Mux<N1>) -> Self { Simulational::Mux1(c) }
}

impl<A: Nat, D: Nat> From<Adder> for Simulational<A, D> {
    fn from(c: Adder) -> Self { Simulational::Adder(c) }
}
