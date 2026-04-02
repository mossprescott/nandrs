use frunk::coproduct::CoprodInjector;
use frunk::{Coprod, hlist};
use simulator::component::Combinational;
use simulator::declare::{BusRef, Interface};

use simulator::{Chip, Flat, IC, Input1, Input16, Output, Output16, Reflect, expand_t, flatten_g};

// Re-export since the other components here parallel Nand:
pub use simulator::component::{Buffer, Nand};

/// Components used and implemented in this project: simple, logical components for 1 and 16 bits.
pub type Project01ComponentT = Coprod!(
    Nand, Buffer, Not, And, Or, Xor, Mux, Dmux, Not16, And16, Mux16
);

/// Recursively expand_t() until only primitives are left.
pub fn flatten_t<C, Idx>(chip: C) -> IC<Combinational>
where
    C: Reflect,
    Project01ComponentT: CoprodInjector<C, Idx>,
{
    flatten_g::<C, Project01ComponentT, Idx, Combinational, _>(
        chip,
        "flat",
        hlist![
            |c: Nand| Flat::Done(vec![Combinational::Nand(c)]),
            |c: Buffer| Flat::Done(vec![Combinational::Buffer(c)]),
            |c: Not| Flat::Continue(c.expand_t()),
            |c: And| Flat::Continue(c.expand_t()),
            |c: Or| Flat::Continue(c.expand_t()),
            |c: Xor| Flat::Continue(c.expand_t()),
            |c: Mux| Flat::Continue(c.expand_t()),
            |c: Dmux| Flat::Continue(c.expand_t()),
            |c: Not16| Flat::Continue(c.expand_t()),
            |c: And16| Flat::Continue(c.expand_t()),
            |c: Mux16| Flat::Continue(c.expand_t()),
        ],
    )
}

/// Inverts its input.
#[derive(Clone, Reflect, Chip)]
pub struct Not {
    pub a: Input1,
    pub out: Output,
}
impl Not {
    expand_t!([Nand], |this| {
        nand: Nand { a: this.a, b: this.a, out: this.out },
    });
}

/// True only when both inputs are true.
#[derive(Clone, Reflect, Chip)]
pub struct And {
    pub a: Input1,
    pub b: Input1,
    pub out: Output,
}
impl And {
    expand_t!([Nand, Not], |this| {
        nand: Nand {
            a: this.a,
            b: this.b,
            out: Output::new(),
        },
        not: Not {
            a: nand.out.into(),
            out: this.out,
        },
    });
}

/// True when at least one input is true.
#[derive(Clone, Reflect, Chip)]
pub struct Or {
    pub a: Input1,
    pub b: Input1,
    pub out: Output,
}
impl Or {
    expand_t!([Not, Nand], |this| {
        not_a: Not  { a: this.a,           out: Output::new() },
        not_b: Not  { a: this.b,           out: Output::new() },
        nand:  Nand { a: not_a.out.into(), b: not_b.out.into(), out: this.out },
    });
}

/// True when inputs differ.
#[derive(Clone, Reflect, Chip)]
pub struct Xor {
    pub a: Input1,
    pub b: Input1,
    pub out: Output,
}
impl Xor {
    expand_t!([Nand], |this| {
        n1:  Nand { a: this.a,        b: this.b,        out: Output::new() },
        n2:  Nand { a: this.a,        b: n1.out.into(), out: Output::new() },
        n3:  Nand { a: this.b,        b: n1.out.into(), out: Output::new() },
        out: Nand { a: n2.out.into(), b: n3.out.into(), out: this.out },
    });
}

/// Passes a0 through when sel is 0, a1 when sel is 1.
#[derive(Clone, Reflect, Chip)]
pub struct Mux {
    pub a0: Input1,
    pub a1: Input1,
    pub sel: Input1,
    pub out: Output,
}
impl Mux {
    expand_t!([Not, Nand], |this| {
        not_sel: Not  { a: this.sel,            out: Output::new() },
        nand0:   Nand { a: not_sel.out.into(),  b: this.a0,          out: Output::new() },
        nand1:   Nand { a: this.sel,            b: this.a1,          out: Output::new() },
        out:     Nand { a: nand0.out.into(),    b: nand1.out.into(), out: this.out },
    });
}

/// Routes input to a when sel is 0, or b when sel is 1; the unused output is zero.
#[derive(Clone, Reflect, Chip)]
pub struct Dmux {
    pub input: Input1,
    pub sel: Input1,
    pub a: Output,
    pub b: Output,
}
impl Dmux {
    expand_t!([Not, And], |this| {
        not_sel: Not { a: this.sel,   out: Output::new() },
        and_a:   And { a: this.input, b: not_sel.out.into(), out: this.a },
        and_b:   And { a: this.input, b: this.sel,           out: this.b },
    });
}

/// Inverts each bit of a 16-bit input.
#[derive(Clone, Reflect, Chip)]
pub struct Not16 {
    pub a: Input16,
    pub out: Output16,
}
impl Not16 {
    expand_t!([Not], |this| {
        for i in 0..16 {
            _not: Not { a: this.a.bit(i), out: this.out.bit(i) }
        }
    });
}

/// Bitwise `And` across two 16-bit inputs.
#[derive(Clone, Reflect, Chip)]
pub struct And16 {
    pub a: Input16,
    pub b: Input16,
    pub out: Output16,
}
impl And16 {
    expand_t!([And], |this| {
        for i in 0..16 {
            _and: And { a: this.a.bit(i), b: this.b.bit(i), out: this.out.bit(i) }
        }
    });
}

/// Selects between two 16-bit inputs bit-by-bit, using a single sel bit.
#[derive(Clone, Reflect, Chip)]
pub struct Mux16 {
    pub a0: Input16,
    pub a1: Input16,
    pub sel: Input1,
    pub out: Output16,
}

impl Mux16 {
    expand_t!([Not, Nand], |this| {
        // Note: saving 15 gates here by sharing not_sel
        not_sel: Not { a: this.sel, out: Output::new() },
        for i in 0..16 {
            nand0: Nand { a: not_sel.out.clone().into(), b: this.a0.bit(i),           out: Output::new() },
            nand1: Nand { a: this.sel,                   b: this.a1.bit(i),           out: Output::new() },
            _out:  Nand { a: nand0.out.clone().into(),   b: nand1.out.clone().into(), out: this.out.bit(i) }
        }
    });
}
