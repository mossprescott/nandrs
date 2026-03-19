use std::collections::HashMap;
use std::marker::PhantomData;
use std::sync::atomic::{AtomicUsize, Ordering};

use crate::nat::{Nat, N1, N16};

static NEXT_ID: AtomicUsize = AtomicUsize::new(1);

/// Unique identity of a wire (bus). Two bus ends with the same `WireId` are connected.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct WireId(pub usize);

impl WireId {
    fn new() -> Self {
        WireId(NEXT_ID.fetch_add(1, Ordering::Relaxed))
    }
}

/// The end of a wire that connects to destination components as one of their inputs.
///
/// Carries a shared bus identity (`id`) and a bit `offset` within that bus.
/// Copies share the same identity, so any number of inputs can read the same wire.
pub struct InputBus<Width: Nat> {
    width: PhantomData<Width>,
    /// Override for the number of bits, when fewer than Width::as_int() are connected.
    /// 0 means "use the type-level width".
    effective_width: usize,
    id: WireId,
    offset: usize,
}
impl<Width: Nat> InputBus<Width> {
    pub fn new() -> Self {
        InputBus { width: PhantomData, effective_width: 0, id: WireId::new(), offset: 0 }
    }

    /// Select a single bit from this bus, returning a 1-bit InputBus that shares
    /// the same underlying wire identity but refers only to bit `i`.
    pub fn bit(&self, i: usize) -> Input {
        assert!(i < Width::as_int(), "bit index {} out of range for {}-bit bus", i, Width::as_int());
        InputBus { width: PhantomData, effective_width: 0, id: self.id, offset: self.offset + i }
    }

    /// Slice `len` bits starting at `offset` from this bus.
    /// The returned bus shares the same wire identity but its BusRef will have width = `len`.
    pub fn mask(&self, offset: usize, len: usize) -> InputBus<Width> {
        assert!(offset + len <= Width::as_int(), "mask({}, {}) out of range for {}-bit bus", offset, len, Width::as_int());
        InputBus { width: PhantomData, effective_width: len, id: self.id, offset: self.offset + offset }
    }
}

/// Copy/Clone need manual impls to avoid requiring `Width: Copy/Clone` (phantom type).
impl<Width: Nat> Copy  for InputBus<Width> {}
impl<Width: Nat> Clone for InputBus<Width> { fn clone(&self) -> Self { *self } }

/// A simple, single-valued input signal; that is, an incoming 1-bit wire.
pub type Input = InputBus<N1>;

/// A multi-bit input signal; that is, an incoming 16-bit bus.
pub type Input16 = InputBus<N16>;

/// The end of a wire that originates from a single component output.
///
/// Carries a shared bus identity (`id`) and a bit `offset` within that bus.
/// Copies share the same identity, enabling fan-out to multiple inputs.
pub struct OutputBus<Width: Nat> {
    width: PhantomData<Width>,
    id: WireId,
    offset: usize,
}

/// Copy/Clone need manual impls to avoid requiring `Width: Copy/Clone` (phantom type).
impl<Width: Nat> Copy  for OutputBus<Width> {}
impl<Width: Nat> Clone for OutputBus<Width> { fn clone(&self) -> Self { *self } }

impl<Width: Nat> From<OutputBus<Width>> for InputBus<Width> {
    /// Any number of inputs can be fed by the same output.
    fn from(output: OutputBus<Width>) -> Self {
        InputBus { width: PhantomData, effective_width: 0, id: output.id, offset: output.offset }
    }
}
impl<Width: Nat> OutputBus<Width> {
    /// Make a new wire of any width.
    pub fn new<N: Nat>() -> OutputBus<N> {
        OutputBus { width: PhantomData, id: WireId::new(), offset: 0 }
    }

    /// Select a single bit from this output bus, returning a 1-bit OutputBus that
    /// shares the same underlying wire identity but refers only to bit `i`.
    pub fn bit(&self, i: usize) -> Output {
        assert!(i < Width::as_int(), "bit index {} out of range for {}-bit bus", i, Width::as_int());
        OutputBus { width: PhantomData, id: self.id, offset: self.offset + i }
    }

    /// Slice `len` bits starting at `offset` from this bus, returning an `InputBus<Width>`
    /// with the same wire identity but a runtime-specified effective width.
    /// Useful for connecting a subset of a wide bus to a narrower address input.
    pub fn mask(&self, offset: usize, len: usize) -> InputBus<Width> {
        assert!(offset + len <= Width::as_int(), "mask({}, {}) out of range for {}-bit bus", offset, len, Width::as_int());
        InputBus { width: PhantomData, effective_width: len, id: self.id, offset: self.offset + offset }
    }
}

/// A simple, single-valued output signal; that is, an outgoing 1-bit wire.
pub type Output = OutputBus<N1>;

/// A multi-bit output signal; that is, an outgoing 16-bit bus.
pub type Output16 = OutputBus<N16>;

pub trait Component {
    type Target;

    /// Define the semantics of a certain Component type, by expanding it on demand, usually to
    /// a larger number of "more primitive" components. When this expansion is applied recursively,
    /// it ultimately produces a completely "flat" set of interconnected primitives.
    ///
    /// If the component is already primitive, then None.
    fn expand(&self) -> Option<IC<Self::Target>>;
}

/// Enumerate the inputs and outputs of a component for reference from the outside.
/// Needed for any component to be analyzed or simulated in a generic way.
///
/// Typically derived.
pub trait Reflect {
    fn reflect(&self) -> Interface;
    fn name(&self) -> String;
}

/// Implemented by components (or wrappers) that may be a Const source.
pub trait AsConst {
    fn as_const(&self) -> Option<u64> { None }
}

/// Construct a fresh instance of a chip struct with new Input/Output buses on every port.
/// This is good for making stand-alone instances, when that's useful for testing.
///
/// Typically derived.
pub trait Chip {
    fn chip() -> Self;
}

/// Type-erased bus reference, for use in Interface where the width is only known at runtime.
/// The `id` field carries the wire identity: two BusRefs with the same `id` refer to
/// the same bus. `offset` is the first bit index within the bus; `width` is the count of bits.
#[derive(Clone, Copy)]
pub struct BusRef {
    pub id: WireId,
    pub offset: usize,
    pub width: usize,
}

impl BusRef {
    pub fn from_input<W: Nat>(input: InputBus<W>) -> Self {
        let width = if input.effective_width != 0 { input.effective_width } else { W::as_int() };
        BusRef { id: input.id, offset: input.offset, width }
    }

    pub fn from_output<W: Nat>(output: OutputBus<W>) -> Self {
        BusRef { id: output.id, offset: output.offset, width: W::as_int() }
    }
}

/// Enumerates the exposed inputs and outputs of some chip or component.
#[derive(Clone)]
pub struct Interface {
    pub inputs: HashMap<String, BusRef>,
    pub outputs: HashMap<String, BusRef>,
}

/// A circuit composed of inputs, outputs, and zero or more components of a certain type.
///
/// Invariant: every input of every component must refer to either: one of the inputs of
/// self.intf, or an output associated with some other component in the same IC.
pub struct IC<C> {
    pub name: String,

    /// The exposed inputs and outputs.
    pub intf: Interface,

    /// The constituent components.
    pub components: Vec<C>,
}

impl<C> Reflect for IC<C> {
    fn reflect(&self) -> Interface {
        self.intf.clone()
    }

    fn name(&self) -> String {
        self.name.clone()
    }
}

/// Generate a `fn expand` body from a record of `name: StructType { fields }` entries.
///
/// ```ignore
/// expand! { |this| {
///     nand: Nand { a: this.a, b: this.b, out: this.out },
/// }}
/// ```
///
/// Use `forward` to declare a wire that can be referenced before its source component:
/// ```ignore
/// expand! { |this| {
///     wire: forward Output::new(),
///     user: Foo { a: wire.into(), out: Output::new() },
///     source: Bar { out: wire },
/// }}
/// ```
///
/// Forward declarations produce a `let` binding but are not pushed as components.
///
/// Use `for` to generate components in a loop (range must use literals):
/// ```ignore
/// expand! { |this| {
///     for i in 0..16 {
///         _not: Nand { a: this.a.bit(i).into(), b: this.a.bit(i).into(), out: this.out.bit(i) },
///     }
/// }}
/// ```
#[macro_export]
macro_rules! expand {
    // Entry point: two passes — all `let` bindings first, then all `push`es.
    ( |$this:ident| { $($body:tt)* } ) => {
        fn expand(&self) -> Option<$crate::IC<Self::Target>> {
            let $this = self;
            $crate::expand!(@lets; $($body)*);
            let mut components = vec![];
            $crate::expand!(@pushes components; $($body)*);
            Some($crate::IC {
                name: $crate::Reflect::name(self),
                intf: $crate::Reflect::reflect(self),
                components,
            })
        }
    };

    // --- Phase 1: emit `let` bindings for all entries ---

    (@lets;) => {};
    (@lets; $var:ident : forward $expr:expr, $($rest:tt)*) => {
        let $var = $expr;
        $crate::expand!(@lets; $($rest)*);
    };
    (@lets; $var:ident : forward $expr:expr) => {
        let $var = $expr;
    };
    // For loops: skip in @lets (handled entirely in @pushes)
    (@lets; for $i:ident in $start:literal .. $end:literal { $($inner:tt)* } $($rest:tt)*) => {
        $crate::expand!(@lets; $($rest)*);
    };
    (@lets; $var:ident : $T:ident { $($fields:tt)* }, $($rest:tt)*) => {
        let $var = $T { $($fields)* };
        $crate::expand!(@lets; $($rest)*);
    };
    (@lets; $var:ident : $T:ident { $($fields:tt)* }) => {
        let $var = $T { $($fields)* };
    };

    // --- Phase 2: emit `push` only for component entries (skip forwards) ---

    (@pushes $components:ident;) => {};
    (@pushes $components:ident; $var:ident : forward $expr:expr, $($rest:tt)*) => {
        $crate::expand!(@pushes $components; $($rest)*);
    };
    (@pushes $components:ident; $var:ident : forward $expr:expr) => {};
    // For loops: construct and push each iteration
    (@pushes $components:ident; for $i:ident in $start:literal .. $end:literal { $($inner:tt)* } $($rest:tt)*) => {
        for $i in $start..$end {
            $crate::expand!(@for_body $components; $($inner)*);
        }
        $crate::expand!(@pushes $components; $($rest)*);
    };
    (@pushes $components:ident; $var:ident : $T:ident { $($fields:tt)* }, $($rest:tt)*) => {
        $components.push($var.into());
        $crate::expand!(@pushes $components; $($rest)*);
    };
    (@pushes $components:ident; $var:ident : $T:ident { $($fields:tt)* }) => {
        $components.push($var.into());
    };

    // --- For loop body: construct and push in one step (loop-scoped) ---

    (@for_body $components:ident;) => {};
    (@for_body $components:ident; $var:ident : $T:ident { $($fields:tt)* }, $($rest:tt)*) => {
        let $var = $T { $($fields)* };
        $components.push($var.into());
        $crate::expand!(@for_body $components; $($rest)*);
    };
    (@for_body $components:ident; $var:ident : $T:ident { $($fields:tt)* }) => {
        let $var = $T { $($fields)* };
        $components.push($var.into());
    };
}
