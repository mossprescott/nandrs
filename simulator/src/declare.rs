use std::collections::HashMap;
use std::marker::PhantomData;
use std::sync::atomic::{AtomicUsize, Ordering};

use crate::nat::{N1, N16, Nat};

static NEXT_ID: AtomicUsize = AtomicUsize::new(1);

/// Unique identity of a wire (bus). Two bus ends with the same `WireId` are connected.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct WireId(pub usize);

impl WireId {
    fn new() -> Self {
        WireId(NEXT_ID.fetch_add(1, Ordering::Relaxed))
    }
}

//
// Input wiring:
//

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
        InputBus {
            width: PhantomData,
            effective_width: 0,
            id: WireId::new(),
            offset: 0,
        }
    }

    /// Select a single bit from this bus, returning a 1-bit InputBus that shares
    /// the same underlying wire identity but refers only to bit `i`.
    pub fn bit(&self, i: usize) -> Input1 {
        assert!(
            i < Width::as_int(),
            "bit index {} out of range for {}-bit bus",
            i,
            Width::as_int()
        );
        Input::Bus(InputBus {
            width: PhantomData,
            effective_width: 0,
            id: self.id,
            offset: self.offset + i,
        })
    }

    /// Slice `len` bits starting at `offset` from this bus.
    /// The returned bus shares the same wire identity but its BusRef will have width = `len`.
    pub fn mask(&self, offset: usize, len: usize) -> InputBus<Width> {
        assert!(
            offset + len <= Width::as_int(),
            "mask({}, {}) out of range for {}-bit bus",
            offset,
            len,
            Width::as_int()
        );
        InputBus {
            width: PhantomData,
            effective_width: len,
            id: self.id,
            offset: self.offset + offset,
        }
    }
}

/// Copy/Clone need manual impls to avoid requiring `Width: Copy/Clone` (phantom type).
impl<Width: Nat> Copy for InputBus<Width> {}
impl<Width: Nat> Clone for InputBus<Width> {
    fn clone(&self) -> Self {
        *self
    }
}

pub enum Input<Width: Nat> {
    /// A constant value, backed by its own WireId so the wire-ref machinery still works.
    Fixed(u64, WireId),

    Bus(InputBus<Width>),
}

impl<Width: Nat> Copy for Input<Width> {}
impl<Width: Nat> Clone for Input<Width> {
    fn clone(&self) -> Self {
        *self
    }
}

/// The most generic input: a bus that may or may not be connected eventually.
impl<Width: Nat> Input<Width> {
    pub fn new() -> Self {
        Input::Bus(InputBus::new())
    }

    fn bus(&self) -> &InputBus<Width> {
        match self {
            Input::Bus(bus) => bus,
            Input::Fixed(..) => panic!("cannot index into a fixed input"),
        }
    }

    pub fn bit(&self, i: usize) -> Input1 {
        assert!(
            i < Width::as_int(),
            "bit index {} out of range for {}-bit input",
            i,
            Width::as_int()
        );
        match self {
            Input::Bus(bus) => bus.bit(i),
            Input::Fixed(value, _) => fixed((value >> i) & 1),
        }
    }

    pub fn mask(&self, offset: usize, len: usize) -> InputBus<Width> {
        self.bus().mask(offset, len)
    }
}

impl<Width: Nat> From<InputBus<Width>> for Input<Width> {
    fn from(bus: InputBus<Width>) -> Self {
        Input::Bus(bus)
    }
}

/// An output can always be the source for another input — whether or not other inputs are
/// connected.
impl<Width: Nat> From<OutputBus<Width>> for Input<Width> {
    fn from(output: OutputBus<Width>) -> Self {
        Input::Bus(InputBus::from(output))
    }
}

/// An input providing fixed bit values.
pub fn fixed<Width: Nat>(value: u64) -> Input<Width> {
    // Better to crash than find out much later that some of your 1 bits got dropped on the floor.
    assert!(value < (1u64 << Width::as_int()));

    Input::Fixed(value, WireId::new())
}

/// A simple, single-valued input signal; that is, an incoming 1-bit wire.
pub type Input1 = Input<N1>;

/// A multi-bit input signal; that is, an incoming 16-bit bus.
pub type Input16 = Input<N16>;

//
// Output wiring:
//

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
impl<Width: Nat> Copy for OutputBus<Width> {}
impl<Width: Nat> Clone for OutputBus<Width> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<Width: Nat> From<OutputBus<Width>> for InputBus<Width> {
    /// Any number of inputs can be fed by the same output.
    fn from(output: OutputBus<Width>) -> Self {
        InputBus {
            width: PhantomData,
            effective_width: 0,
            id: output.id,
            offset: output.offset,
        }
    }
}
impl<Width: Nat> OutputBus<Width> {
    /// Make a new wire of any width.
    pub fn new<N: Nat>() -> OutputBus<N> {
        OutputBus {
            width: PhantomData,
            id: WireId::new(),
            offset: 0,
        }
    }

    /// Select a single bit from this output bus, returning a 1-bit OutputBus that
    /// shares the same underlying wire identity but refers only to bit `i`.
    pub fn bit(&self, i: usize) -> Output {
        assert!(
            i < Width::as_int(),
            "bit index {} out of range for {}-bit bus",
            i,
            Width::as_int()
        );
        OutputBus {
            width: PhantomData,
            id: self.id,
            offset: self.offset + i,
        }
    }

    /// Slice `len` bits starting at `offset` from this bus, returning an `InputBus<Width>`
    /// with the same wire identity but a runtime-specified effective width.
    /// Useful for connecting a subset of a wide bus to a narrower address input.
    pub fn mask(&self, offset: usize, len: usize) -> InputBus<Width> {
        assert!(
            offset + len <= Width::as_int(),
            "mask({}, {}) out of range for {}-bit bus",
            offset,
            len,
            Width::as_int()
        );
        InputBus {
            width: PhantomData,
            effective_width: len,
            id: self.id,
            offset: self.offset + offset,
        }
    }
}

/// A simple, single-valued output signal; that is, an outgoing 1-bit wire.
// TODO: rename to Output1 for consistency
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
    /// When Some, this input is a compile-time constant. The WireId is valid but needs to be
    /// seeded with this value before evaluation.
    pub fixed: Option<u64>,
}

impl BusRef {
    pub fn from_input_bus<W: Nat>(input: InputBus<W>) -> Self {
        let width = if input.effective_width != 0 {
            input.effective_width
        } else {
            W::as_int()
        };
        BusRef {
            id: input.id,
            offset: input.offset,
            width,
            fixed: None,
        }
    }

    pub fn from_input<W: Nat>(input: Input<W>) -> Self {
        match input {
            Input::Bus(bus) => Self::from_input_bus(bus),
            Input::Fixed(value, id) => BusRef {
                id,
                offset: 0,
                width: W::as_int(),
                fixed: Some(value),
            },
        }
    }

    pub fn from_output<W: Nat>(output: OutputBus<W>) -> Self {
        BusRef {
            id: output.id,
            offset: output.offset,
            width: W::as_int(),
            fixed: None,
        }
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

impl<C> IC<C> {
    pub fn map<D>(&self, f: impl FnMut(C) -> D) -> IC<D>
    where
        C: Clone,
    {
        IC {
            name: self.name.clone(),
            intf: self.intf.clone(),
            components: self.components.iter().cloned().map(f).collect(),
        }
    }
}

impl<C> Reflect for IC<C> {
    fn reflect(&self) -> Interface {
        self.intf.clone()
    }

    fn name(&self) -> String {
        self.name.clone()
    }
}

/// Blanket `Reflect` for frunk `Coproduct`/`CNil`: delegates to whichever variant is active.
impl Reflect for frunk::coproduct::CNil {
    fn reflect(&self) -> Interface {
        unreachable!()
    }
    fn name(&self) -> String {
        unreachable!()
    }
}

impl<Head: Reflect, Tail: Reflect> Reflect for frunk::Coproduct<Head, Tail> {
    fn reflect(&self) -> Interface {
        match self {
            frunk::Coproduct::Inl(h) => h.reflect(),
            frunk::Coproduct::Inr(t) => t.reflect(),
        }
    }
    fn name(&self) -> String {
        match self {
            frunk::Coproduct::Inl(h) => h.name(),
            frunk::Coproduct::Inr(t) => t.name(),
        }
    }
}

/// Generate a typed `expand_t` method for a chip.
///
/// Takes a bracketed list of target component types and a body. Generates a
/// generic method `expand_t<C, T1Idx, T2Idx, ...>(&self) -> IC<C>` with one `CoprodInjector`
/// bound per listed type, and uses `C::inject(component)` to build the result.
///
/// ```ignore
/// expand_t!([Nand, Not], |this| {
///     nand: Nand { a: this.a, b: this.b, out: Output::new() },
///     not:  Not  { a: nand.out.into(),   out: this.out },
/// });
/// ```
#[macro_export]
macro_rules! expand_t {
    // Entry point: generate the fn signature via paste, then the body.
    ([$($T:ident),+], |$this:ident| { $($body:tt)* }) => {
        $crate::paste::paste! {
            pub fn expand_t<C, $([<$T Idx>]),+>(&self) -> $crate::IC<C>
            where
                $(C: ::frunk::coproduct::CoprodInjector<$T, [<$T Idx>]>,)+
            {
                let $this = self;
                let mut __components = vec![];
                $crate::expand_t!(@lets __components; $($body)*);
                $crate::expand_t!(@pushes __components; $($body)*);
                $crate::IC {
                    name: $crate::Reflect::name(self),
                    intf: $crate::Reflect::reflect(self),
                    components: __components,
                }
            }
        }
    };

    // --- Phase 1: emit `let` bindings ---

    (@lets $c:ident;) => {};
    // For loops: construct and push during @lets (while bindings are alive)
    (@lets $c:ident; for $i:ident in $start:literal .. $end:literal { $($inner:tt)* } $($rest:tt)*) => {
        for $i in $start..$end {
            $crate::expand_t!(@for_body $c; $($inner)*);
        }
        $crate::expand_t!(@lets $c; $($rest)*);
    };
    // Fold: collect injected components into a saved vec; extend into $c during @pushes.
    (@lets $c:ident; $var:ident : ($start:literal .. $end:literal) . fold ($init:expr, | $acc:ident, $i:ident | { $($body:tt)* }) , $($rest:tt)*) => {
        let $var = {
            let mut __fold_tmp = vec![];
            let _ = ($start..$end).fold($init, |$acc, $i| {
                $crate::expand_t!(@fold_bind $($body)*);
                let __fold_next = { $crate::expand_t!(@fold_accum $($body)*) };
                $crate::expand_t!(@fold_push __fold_tmp; $($body)*);
                __fold_next
            });
            __fold_tmp
        };
        $crate::expand_t!(@lets $c; $($rest)*);
    };
    (@lets $c:ident; $var:ident : ($start:literal .. $end:literal) . fold ($init:expr, | $acc:ident, $i:ident | { $($body:tt)* })) => {
        let $var = {
            let mut __fold_tmp = vec![];
            let _ = ($start..$end).fold($init, |$acc, $i| {
                $crate::expand_t!(@fold_bind $($body)*);
                let __fold_next = { $crate::expand_t!(@fold_accum $($body)*) };
                $crate::expand_t!(@fold_push __fold_tmp; $($body)*);
                __fold_next
            });
            __fold_tmp
        };
    };
    (@lets $c:ident; $var:ident : forward $expr:expr, $($rest:tt)*) => {
        let $var = $expr;
        $crate::expand_t!(@lets $c; $($rest)*);
    };
    (@lets $c:ident; $var:ident : forward $expr:expr) => {
        let $var = $expr;
    };
    (@lets $c:ident; $var:ident : $T:ident { $($fields:tt)* }, $($rest:tt)*) => {
        let $var = $T { $($fields)* };
        $crate::expand_t!(@lets $c; $($rest)*);
    };
    (@lets $c:ident; $var:ident : $T:ident { $($fields:tt)* }) => {
        let $var = $T { $($fields)* };
    };

    // --- Phase 2: push via C::inject (skip for loops and folds, already handled during @lets) ---

    (@pushes $c:ident;) => {};
    (@pushes $c:ident; $var:ident : forward $expr:expr, $($rest:tt)*) => {
        $crate::expand_t!(@pushes $c; $($rest)*);
    };
    (@pushes $c:ident; $var:ident : forward $expr:expr) => {};
    (@pushes $c:ident; for $i:ident in $start:literal .. $end:literal { $($inner:tt)* } $($rest:tt)*) => {
        $crate::expand_t!(@pushes $c; $($rest)*);
    };
    // Folds: extend with saved vec (collected during @lets)
    (@pushes $c:ident; $var:ident : ($start:literal .. $end:literal) . fold ($init:expr, | $acc:ident, $i:ident | { $($inner:tt)* }) , $($rest:tt)*) => {
        $c.extend($var);
        $crate::expand_t!(@pushes $c; $($rest)*);
    };
    (@pushes $c:ident; $var:ident : ($start:literal .. $end:literal) . fold ($init:expr, | $acc:ident, $i:ident | { $($inner:tt)* })) => {
        $c.extend($var);
    };
    (@pushes $c:ident; $var:ident : $T:ident { $($fields:tt)* }, $($rest:tt)*) => {
        $c.push(C::inject($var));
        $crate::expand_t!(@pushes $c; $($rest)*);
    };
    (@pushes $c:ident; $var:ident : $T:ident { $($fields:tt)* }) => {
        $c.push(C::inject($var));
    };

    // --- For loop body: construct, clone+inject, repeat ---

    (@for_body $c:ident;) => {};
    (@for_body $c:ident; $var:ident : $T:ident { $($fields:tt)* }, $($rest:tt)*) => {
        let $var = $T { $($fields)* };
        $c.push(C::inject($var.clone()));
        $crate::expand_t!(@for_body $c; $($rest)*);
    };
    (@for_body $c:ident; $var:ident : $T:ident { $($fields:tt)* }) => {
        let $var = $T { $($fields)* };
        $c.push(C::inject($var));
    };

    // --- Fold sub-phases ---

    // @fold_bind: create let bindings for each component entry, skip the final accumulator
    (@fold_bind $var:ident : $T:ident { $($fields:tt)* }, $($rest:tt)*) => {
        let $var = $T { $($fields)* };
        $crate::expand_t!(@fold_bind $($rest)*);
    };
    (@fold_bind $next:expr) => {};

    // @fold_accum: skip component entries, return the final accumulator expression
    (@fold_accum $var:ident : $T:ident { $($fields:tt)* }, $($rest:tt)*) => {
        $crate::expand_t!(@fold_accum $($rest)*)
    };
    (@fold_accum $next:expr) => { $next };

    // @fold_push: inject each component entry, skip the final accumulator
    (@fold_push $c:ident; $var:ident : $T:ident { $($fields:tt)* }, $($rest:tt)*) => {
        $c.push(C::inject($var));
        $crate::expand_t!(@fold_push $c; $($rest)*);
    };
    (@fold_push $c:ident; $next:expr) => {};
}
