use crate::project_01::{And, And16, Buffer, Mux, Mux16, Nand, Not, Not16, Or};
use frunk::coproduct::CoprodInjector;
use frunk::{Coprod, hlist};
use simulator::component::native;
use simulator::component::{Combinational, Computational};
use simulator::declare::{BusRef, Interface};
use simulator::nat::N16;
use simulator::{
    self, Chip, Flat, IC, Input1, Input16, Output, Output16, Reflect, expand_t, fixed, flatten_g,
};

pub type Project02ComponentT = Coprod!(
    Nand, Buffer, Not, And, Or, Mux, Mux16, Not16, And16, HalfAdder, FullAdder, Inc16, Add16,
    Nand16Way, Zero16, Neg16, ALU
);

/// Recursively expand_t() until only primitives are left.
pub fn flatten_t<C, Idx>(chip: C) -> IC<Combinational>
where
    C: Reflect,
    Project02ComponentT: CoprodInjector<C, Idx>,
{
    flatten_g::<C, Project02ComponentT, Idx, Combinational, _>(
        chip,
        "flat",
        hlist![
            |c: Nand| Flat::Done(vec![Combinational::Nand(c)]),
            |c: Buffer| Flat::Done(vec![Combinational::Buffer(c)]),
            |c: Not| Flat::Continue(c.expand_t()),
            |c: And| Flat::Continue(c.expand_t()),
            |c: Or| Flat::Continue(c.expand_t()),
            |c: Mux| Flat::Continue(c.expand_t()),
            |c: Mux16| Flat::Continue(c.expand_t()),
            |c: Not16| Flat::Continue(c.expand_t()),
            |c: And16| Flat::Continue(c.expand_t()),
            |c: HalfAdder| Flat::Continue(c.expand_t()),
            |c: FullAdder| Flat::Continue(c.expand_t()),
            |c: Inc16| Flat::Continue(c.expand_t()),
            |c: Add16| Flat::Continue(c.expand_t()),
            |c: Nand16Way| Flat::Continue(c.expand_t()),
            |c: Zero16| Flat::Continue(c.expand_t()),
            |c: Neg16| Flat::Continue(c.expand_t()),
            |c: ALU| Flat::Continue(c.expand_t()),
        ],
    )
}
/// Like `flatten`, but replaces HalfAdder/FullAdder with native Adder and Mux/Mux16 with
/// native Mux for efficient simulation.
///
/// Note: this is pinned to N16, just so it can rewrite the Mux16 component as a native Mux.
pub fn flatten_for_simulation<C, Idx>(chip: C) -> IC<native::Simulational<N16, N16>>
where
    C: Reflect,
    Project02ComponentT: CoprodInjector<C, Idx>,
{
    flatten_g::<C, Project02ComponentT, Idx, native::Simulational<N16, N16>, _>(
        chip,
        "flat/sim",
        hlist![
            |c: Nand| Flat::Done(vec![Computational::Nand(c).into()]),
            |c: Buffer| Flat::Done(vec![Computational::Buffer(c).into()]),
            |c: Not| Flat::Continue(c.expand_t()),
            |c: And| Flat::Continue(c.expand_t()),
            |c: Or| Flat::Continue(c.expand_t()),
            |c: Mux| Flat::Done(vec![
                native::Mux {
                    a0: c.a0,
                    a1: c.a1,
                    sel: c.sel,
                    out: c.out,
                }
                .into()
            ]),
            |c: Mux16| Flat::Done(vec![
                native::Mux {
                    a0: c.a0,
                    a1: c.a1,
                    sel: c.sel,
                    out: c.out,
                }
                .into()
            ]),
            |c: Not16| Flat::Continue(c.expand_t()),
            |c: And16| Flat::Continue(c.expand_t()),
            |c: HalfAdder| {
                // Tricky: the simulator looks for the carry chain to always pass the carry bit in
                // c, so it's important for the zero bit to go to b here, even though in principle
                // it doesn't matter.
                Flat::Done(vec![
                    native::Adder {
                        a: c.a,
                        b: fixed(0),
                        c: c.b,
                        sum: c.sum,
                        carry: c.carry,
                    }
                    .into(),
                ])
            },
            |c: FullAdder| Flat::Done(vec![
                native::Adder {
                    a: c.a,
                    b: c.b,
                    c: c.c,
                    sum: c.sum,
                    carry: c.carry,
                }
                .into()
            ]),
            |c: Inc16| Flat::Continue(c.expand_t()),
            |c: Add16| Flat::Continue(c.expand_t()),
            |c: Nand16Way| Flat::Continue(c.expand_t()),
            |c: Zero16| Flat::Continue(c.expand_t()),
            |c: Neg16| Flat::Continue(c.expand_t()),
            |c: ALU| Flat::Continue(c.expand_t()),
        ],
    )
}

/// sum = 1s-digit of two-bit sum, carry = 2s-digit
///
/// Note: for efficiency in simulation, there is an alternative expansion for this component to
/// native::Adder.
#[derive(Clone, Reflect, Chip)]
pub struct HalfAdder {
    pub a: Input1,
    pub b: Input1,
    pub sum: Output,
    pub carry: Output,
}

impl HalfAdder {
    /*
    Equivalent to:
      sum = Xor { a = inputs.a, b: inputs.b }
      carry = And {a = inputs.a, b: inputs.b}
    but flattened to use only 5 Nands.
     */
    expand_t!([Nand], |this| {
        // n1 = NAND(a,b) is shared: XOR reuses it, carry = NOT(n1) = NAND(n1,n1)
        n1:    Nand { a: this.a,        b: this.b,        out: Output::new() },
        n2:    Nand { a: this.a,        b: n1.out.into(), out: Output::new() },
        n3:    Nand { a: this.b,        b: n1.out.into(), out: Output::new() },
        sum:   Nand { a: n2.out.into(), b: n3.out.into(), out: this.sum },
        carry: Nand { a: n1.out.into(), b: n1.out.into(), out: this.carry },
    });
}

/// sum = 1s-digit of three-bit sum, carry = 2s-digit
///
/// Note: for efficiency in simulation, there is an alternative expansion for this component to
/// native::Adder.
#[derive(Clone, Reflect, Chip)]
pub struct FullAdder {
    pub a: Input1,
    pub b: Input1,
    pub c: Input1,
    pub sum: Output,
    pub carry: Output,
}

impl FullAdder {
    /*
    Some sharing of common gates to get down to the minimal 9 gates.
    */
    expand_t!([Nand], |this| {
        // n4 = XOR(a,b); n5 = NAND(c, n4) shared by sum and carry paths
        n1:    Nand { a: this.a,        b: this.b,        out: Output::new() },
        n2:    Nand { a: this.a,        b: n1.out.into(), out: Output::new() },
        n3:    Nand { a: this.b,        b: n1.out.into(), out: Output::new() },
        n4:    Nand { a: n2.out.into(), b: n3.out.into(), out: Output::new() },
        n5:    Nand { a: this.c,        b: n4.out.into(), out: Output::new() },
        n6:    Nand { a: this.c,        b: n5.out.into(), out: Output::new() },
        n7:    Nand { a: n4.out.into(), b: n5.out.into(), out: Output::new() },
        sum:   Nand { a: n6.out.into(), b: n7.out.into(), out: this.sum },
        carry: Nand { a: n1.out.into(), b: n5.out.into(), out: this.carry },
    });
}

/// out = in + 1 (16-bit, overflow ignored)
#[derive(Clone, Reflect, Chip)]
pub struct Inc16 {
    pub a: Input16,
    pub out: Output16,
}

impl Inc16 {
    expand_t!([Not, HalfAdder], |this| {
        // bit 0: out[0] = NOT(a[0]); carry = a[0] (the carry-in is implicitly 1)
        not0: Not { a: this.a.bit(0), out: this.out.bit(0) },
        // Carry-ripple: fold threads the carry across iterations
        _carry_out: (1..16).fold(this.a.bit(0), |carry, i| {
            add: HalfAdder {
                a: this.a.bit(i),
                b: carry,
                sum: this.out.bit(i),
                carry: Output::new(),
            },
            add.carry.into()
        }),
    });
}

/// out = a + b (16-bit, overflow ignored)
#[derive(Clone, Reflect, Chip)]
pub struct Add16 {
    pub a: Input16,
    pub b: Input16,
    pub out: Output16,
}

impl Add16 {
    expand_t!([FullAdder], |this| {
        // bit 0: half-add (carry-in is 0)
        add0: FullAdder {
            a: this.a.bit(0),
            b: this.b.bit(0),
            c: fixed(0),
            sum: this.out.bit(0),
            carry: Output::new(),
        },
        // Carry-ripple: fold threads the carry across remaining bits
        _carry_out: (1..16).fold(add0.carry.into(), |carry, i| {
            add: FullAdder {
                a: this.a.bit(i),
                b: this.b.bit(i),
                c: carry,
                sum: this.out.bit(i),
                carry: Output::new(),
            },
            add.carry.into()
        }),
    });
}

/// True if any of the 16 bits is false, as if they were all fed into a big 16-input Nand gate —
/// which is a thing which exists — but here implemented with a series of discrete Ands.
///
/// Note: the simulator will recognize a series of Ands like this and reduce it to a single
/// operation.
#[derive(Clone, Reflect, Chip)]
pub struct Nand16Way {
    pub a: Input16,
    pub out: Output,
}

impl Nand16Way {
    expand_t!([And, Not], |this| {
        // Level 1
        and_01:   And { a: this.a.bit(0).into(),  b: this.a.bit(1).into(),  out: Output::new() },
        and_23:   And { a: this.a.bit(2).into(),  b: this.a.bit(3).into(),  out: Output::new() },
        and_45:   And { a: this.a.bit(4).into(),  b: this.a.bit(5).into(),  out: Output::new() },
        and_67:   And { a: this.a.bit(6).into(),  b: this.a.bit(7).into(),  out: Output::new() },
        and_89:   And { a: this.a.bit(8).into(),  b: this.a.bit(9).into(),  out: Output::new() },
        and_ab:   And { a: this.a.bit(10).into(), b: this.a.bit(11).into(), out: Output::new() },
        and_cd:   And { a: this.a.bit(12).into(), b: this.a.bit(13).into(), out: Output::new() },
        and_ef:   And { a: this.a.bit(14).into(), b: this.a.bit(15).into(), out: Output::new() },
        // Level 2
        and_0123: And { a: and_01.out.into(),     b: and_23.out.into(),     out: Output::new() },
        and_4567: And { a: and_45.out.into(),     b: and_67.out.into(),     out: Output::new() },
        and_89ab: And { a: and_89.out.into(),     b: and_ab.out.into(),     out: Output::new() },
        and_cdef: And { a: and_cd.out.into(),     b: and_ef.out.into(),     out: Output::new() },
        // Level 3
        and_lo:   And { a: and_0123.out.into(),   b: and_4567.out.into(),   out: Output::new() },
        and_hi:   And { a: and_89ab.out.into(),   b: and_cdef.out.into(),   out: Output::new() },
        // Level 4
        and_all: And { a: and_lo.out.into(),      b: and_hi.out.into(),     out: Output::new() },

        _not: Not { a: and_all.out.into(), out: this.out },
    });
}

/// Returns 1 if all bits of input are 0.
#[derive(Clone, Reflect, Chip)]
pub struct Zero16 {
    pub a: Input16,
    pub out: Output,
}

impl Zero16 {
    expand_t!([Not16, Nand16Way, Not], |this| {
        // Negate into a single bus; the simulator makes this parallel.
        not: Not16 { a: this.a, out: Output16::new() },

        // Compare them all at once, as if 16-way fan-in was a thing. The simulator
        // handles this efficiently, too.
        nand_all: Nand16Way { a: not.out.into(), out: Output::new() },

        _f: Not { a: nand_all.out.into(), out: this.out },
    });
}

/// out = true if the most-significant bit of a is 1 (i.e., input is negative in two's complement).
#[derive(Clone, Reflect, Chip)]
pub struct Neg16 {
    pub a: Input16,
    pub out: Output,
}

impl Neg16 {
    /*
     out = a[15]
    */
    expand_t!([Buffer], |this| {
        sign: Buffer { a: this.a.bit(15), out: this.out },
    });
}

/// Hack ALU: computes one of several functions of x and y selected by control bits.
#[derive(Clone, Reflect, Chip)]
pub struct ALU {
    /// "Left" input
    pub x: Input16,
    // "Right" input
    pub y: Input16,
    /// Zero the x input
    pub zx: Input1,
    /// Negate the x input (i.e. "not")
    pub nx: Input1,
    /// Zero the y input
    pub zy: Input1,
    /// Negate the y input (i.e. "not")
    pub ny: Input1,
    /// 0 => x && y; 1 => x + y
    pub f: Input1,
    /// Negate the result (i.e. "not")
    pub no: Input1,

    /// 16-bit result
    pub out: Output16,
    /// Flag: is the result equal to zero (all bits zero)
    pub zr: Output,
    /// Flag: is the result < 0? (high bit set)
    pub ng: Output,

    /// Disable: when 1, all outputs are forced to 0. This makes it easier for the simulator to
    /// identify this logic as inactive and avoid spending time evaluating it.
    pub disable: Input1,
}

impl ALU {
    expand_t!([Mux16, Not16, And16, Add16, Mux, Zero16, Neg16, Not, And], |this| {
         // zx/nx: conditionally zero then negate x
         x1: Mux16 { sel: this.zx, a0: this.x, a1: fixed(0), out: Output16::new() },
         x2_not: Not16 { a: x1.out.into(), out: Output16::new() },
         x2: Mux16 { sel: this.nx, a0: x1.out.into(), a1: x2_not.out.into(), out: Output16::new() },

         // zy/ny: conditionally zero then negate y
         y1: Mux16 { sel: this.zy, a0: this.y, a1: fixed(0), out: Output16::new() },
         y2_not: Not16 { a: y1.out.into(), out: Output16::new() },
         y2: Mux16 { sel: this.ny, a0: y1.out.into(), a1: y2_not.out.into(), out: Output16::new() },

         and: And16 { a: x2.out.into(), b: y2.out.into(), out: Output16::new() },

         // Gate Add16 inputs: only active when f=1 AND !disable. Mux16 gives Add16
         // its own copy of the inputs, breaking the sharing with And16 so the nesting
         // algorithm can move all Add16 nands into a mux branch.
         not_disable: Not { a: this.disable, out: Output::new() },
         add_active: And { a: this.f, b: not_disable.out.into(), out: Output::new() },
         add_x: Mux16 { sel: add_active.out.into(), a0: fixed(0), a1: x2.out.into(), out: Output16::new() },
         add_y: Mux16 { sel: add_active.out.into(), a0: fixed(0), a1: y2.out.into(), out: Output16::new() },
         add: Add16 { a: add_x.out.into(), b: add_y.out.into(), out: Output16::new() },

         result: Mux16 { sel: this.f, a0: and.out.into(), a1: add.out.into(), out: Output16::new() },
         rn: Not16 { a: result.out.into(), out: Output16::new() },
         raw_out: Mux16 { sel: this.no, a0: result.out.into(), a1: rn.out.into(), out: Output16::new() },

         // Compute zr from raw_out (before the disable gate) so it can be nested
         // into the disable mux branch along with the rest of the ALU chain.
         raw_zr: Zero16 { a: raw_out.out.into(), out: Output::new() },

         // Gate output and zr with disable.  When disabled: out=0, zr=1.
         out_gate: Mux16 { sel: this.disable, a0: raw_out.out.into(), a1: fixed(0), out: this.out },
         zr_gate: Mux { sel: this.disable, a0: raw_zr.out.into(), a1: fixed(1), out: this.zr },

         // ng reads from the gated output; when disabled out=0 so ng=0 (correct).
         // Neg16 is 0 nands (just a buffer) so there's nothing to skip.
         rneg: Neg16 { a: this.out.into(), out: this.ng },
    });
}
