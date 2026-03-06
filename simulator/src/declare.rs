use std::collections::HashMap;
use std::marker::PhantomData;
use std::rc::Rc;

use crate::nat::{Nat, N1, N16};

/// The end of a wire that connects to destination components as one of their inputs.
///
/// Carries a shared bus identity (`id`) and a bit `offset` within that bus.
/// Clones share the same identity, so any number of inputs can read the same wire.
#[derive(Clone)]
pub struct InputBus<Width: Nat> {
    width: PhantomData<Width>,
    id: Rc<()>,
    offset: usize,
}
impl<Width: Nat> InputBus<Width> {
    pub fn new() -> Self {
        InputBus { width: PhantomData, id: Rc::new(()), offset: 0 }
    }

    /// Select a single bit from this bus, returning a 1-bit InputBus that shares
    /// the same underlying wire identity but refers only to bit `i`.
    pub fn bit(&self, i: usize) -> Input {
        assert!(i < Width::as_int(), "bit index {} out of range for {}-bit bus", i, Width::as_int());
        InputBus { width: PhantomData, id: self.id.clone(), offset: self.offset + i }
    }
}

/// A simple, single-valued input signal; that is, an incoming 1-bit wire.
pub type Input = InputBus<N1>;

/// A multi-bit input signal; that is, an incoming 16-bit bus.
pub type Input16 = InputBus<N16>;

/// The end of a wire that originates from a single component output.
///
/// Carries a shared bus identity (`id`) and a bit `offset` within that bus.
/// Clones share the same identity, enabling fan-out to multiple inputs.
#[derive(Clone)]
pub struct OutputBus<Width: Nat> {
    width: PhantomData<Width>,
    id: Rc<()>,
    offset: usize,
}
impl<Width: Nat> From<OutputBus<Width>> for InputBus<Width> {
    /// Any number of inputs can be fed by the same output.
    fn from(output: OutputBus<Width>) -> Self {
        InputBus { width: PhantomData, id: output.id, offset: output.offset }
    }
}
impl<Width: Nat> OutputBus<Width> {
    /// Select a single bit from this output bus, returning a 1-bit OutputBus that
    /// shares the same underlying wire identity but refers only to bit `i`.
    pub fn bit(&self, i: usize) -> Output {
        assert!(i < Width::as_int(), "bit index {} out of range for {}-bit bus", i, Width::as_int());
        OutputBus { width: PhantomData, id: self.id.clone(), offset: self.offset + i }
    }
}

/// A simple, single-valued output signal; that is, an outgoing 1-bit wire.
pub type Output = OutputBus<N1>;
impl Output {
    /// Make a new wire of any width.
    pub fn new<N: Nat>() -> OutputBus<N> {
        OutputBus { width: PhantomData, id: Rc::new(()), offset: 0 }
    }
}

/// A multi-bit output signal; that is, an outgoing 16-bit bus.
pub type Output16 = OutputBus<N16>;

pub trait Component {
    type Target;

    /// Define the semantics of a certain Component type, by expanding it on demand, usually to
    /// a larger number of "more primitive" components. When this expansion is applied recursively,
    /// it ultimately produces a completely "flat" set of interconnected primitives.
    ///
    /// If the component is already primitive, then None.
    fn expand(&self) -> Option<Vec<Self::Target>>;
}

/// Enumerate the inputs and outputs of a component for reference from the outside.
/// Needed for any component to be analyzed or simulated in a generic way.
pub trait Reflect {
    fn reflect(&self) -> Interface;
    fn name(&self) -> &'static str;
}

/// Type-erased bus reference, for use in Interface where the width is only known at runtime.
/// The `id` field carries the wire identity: two BusRefs with the same `id` pointer refer to
/// the same bus. `offset` is the first bit index within the bus; `width` is the count of bits.
pub struct BusRef {
    pub id: Rc<()>,
    pub offset: usize,
    pub width: usize,
}

impl<Width: Nat> From<InputBus<Width>> for BusRef {
    fn from(input: InputBus<Width>) -> Self {
        BusRef { id: input.id, offset: input.offset, width: Width::as_int() }
    }
}

impl<Width: Nat> From<OutputBus<Width>> for BusRef {
    fn from(output: OutputBus<Width>) -> Self {
        BusRef { id: output.id, offset: output.offset, width: Width::as_int() }
    }
}

pub struct Interface {
    pub inputs: HashMap<String, BusRef>,
    pub outputs: HashMap<String, BusRef>,
}
