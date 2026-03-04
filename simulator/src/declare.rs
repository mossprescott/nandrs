use std::collections::HashMap;
use std::marker::PhantomData;
use std::rc::Rc;

use crate::nat::{Nat, N1, N16};

/// Reference identifying a particular wire/bus.
///
/// Uses Rc<()> as the identity: clones share the same identity, which is how
/// OutputBus → InputBus connections are tracked through the circuit.
#[derive(Clone)]
pub struct Bus<Width: Nat> {
    width: PhantomData<Width>,
    id: Rc<()>,
}
impl<Width: Nat> Bus<Width> {
    /// Not public! See Output::new().
    fn new() -> Bus<Width> {
        Bus { width: PhantomData, id: Rc::new(()) }
    }
}

/// The end of a wire that connects to a single destination component as one of its inputs.
///
/// Note: a single wire/bus can connect to any number of destinations (unlimited fan-out).
/// Wait, is that backwards?
#[derive(Clone)]
pub struct InputBus<Width: Nat> {
    wire: Rc<Bus<Width>>,
}
impl<Width: Nat> InputBus<Width> {
    pub fn new() -> Self {
        InputBus { wire: Rc::new(Bus::new()) }
    }
}

/// A simple, single-valued input signal; that is, an incoming wire.
pub type Input = InputBus<N1>;

/// A multi-bit input signal; that is, an incoming 16-bit bus.
pub type Input16 = InputBus<N16>;

/// The end of a wire that connects to a single origin component.
///
/// Note: any number of wires/buses can connect to a single output (unlimited fan-out).
/// Wait, is that backwards?
#[derive(Clone)]
pub struct OutputBus<Width: Nat> {
    wire: Bus<Width>,
}
impl<Width: Nat> From<OutputBus<Width>> for InputBus<Width> {
    /// Any number of inputs can be fed by the same output.
    fn from(output: OutputBus<Width>) -> Self {
        InputBus { wire: Rc::new(output.wire) }
    }
}

/// A simple, single-valued ouput signal; that is, an outgoing wire.
pub type Output = OutputBus<N1>;
impl Output {
    /// Make a new wire;
    pub fn new<N: Nat>() -> OutputBus<N> {
        OutputBus { wire: Bus::new() }
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

    /// Enumerate the inputs and outputs of the component for reference from the outside. This is
    /// needed for any component to analyzed or simulated in a generic way.
    fn reflect(&self) -> Interface;
}



/// Type-erased bus reference, for use in Interface where the width is only known at runtime.
/// The `id` field carries the wire identity: two BusRefs with the same `id` pointer are the
/// same wire.
pub struct BusRef {
    pub id: Rc<()>,
    pub width: usize,
}

impl<Width: Nat> From<InputBus<Width>> for BusRef {
    fn from(input: InputBus<Width>) -> Self {
        BusRef { id: input.wire.id.clone(), width: Width::as_int() }
    }
}

impl<Width: Nat> From<OutputBus<Width>> for BusRef {
    fn from(output: OutputBus<Width>) -> Self {
        BusRef { id: output.wire.id.clone(), width: Width::as_int() }
    }
}

pub struct Interface {
    pub inputs: HashMap<String, BusRef>,
    pub outputs: HashMap<String, BusRef>,
}
