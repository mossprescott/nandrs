#![allow(unused_variables, dead_code, unused_imports)]

use crate::project_01::{
    And, And16, Buffer, Mux, Mux16, Nand, Not, Not16, Or, Project01Component, Xor,
};
use simulator::Chip as _;
use simulator::Reflect as _;
use simulator::component::Combinational;
use simulator::component::native;
use simulator::declare::{BusRef, Interface};
use simulator::nat::{N16, Nat};
use simulator::{
    self, Chip, Component, IC, Input1, Input16, Output, Output16, Reflect, expand, fixed,
};

#[derive(Clone, Reflect, Component)]
pub enum Project02Component {
    #[delegate]
    Project01(Project01Component),
    HalfAdder(HalfAdder),
    FullAdder(FullAdder),
    Inc16(Inc16),
    Add16(Add16),
    Nand16Way(Nand16Way),
    Zero16(Zero16),
    Neg16(Neg16),
    ALU(ALU),
}

/// Recursively expand until only primitives are left.
pub fn flatten<C: Reflect + Into<Project02Component>>(chip: C) -> IC<Combinational> {
    fn go(comp: Project02Component) -> Vec<Combinational> {
        match comp.expand() {
            None => match comp {
                Project02Component::Project01(p) => crate::project_01::flatten(p).components,
                _ => panic!("Did not reduce to primitive: {:?}", comp.name()),
            },
            Some(ic) => ic.components.into_iter().flat_map(go).collect(),
        }
    }
    IC {
        name: format!("{} (flat)", chip.name()),
        intf: chip.reflect(),
        components: go(chip.into()),
    }
}

/// Like `flatten`, but replaces HalfAdder/FullAdder with native Adder and Mux/Mux16 with
/// native Mux for efficient simulation.
///
/// Note: this is pinned to N16, just so it can rewrite the Mux16 component as a native Mux.
pub fn flatten_for_simulation<C>(chip: C) -> IC<native::Simulational<N16, N16>>
where
    C: Reflect + Into<Project02Component>,
{
    fn go(comp: Project02Component) -> Vec<native::Simulational<N16, N16>> {
        match comp {
            Project02Component::HalfAdder(c) => vec![
                // Tricky: the simulator looks for the carry chain to always pass the carry bit in
                // c, so it's important for the zero bit to go to b here, even though in principle
                // it doesn't matter.
                native::Adder {
                    a: c.a,
                    b: fixed(0),
                    c: c.b,
                    sum: c.sum,
                    carry: c.carry,
                }
                .into(),
            ],
            Project02Component::FullAdder(c) => vec![
                native::Adder {
                    a: c.a,
                    b: c.b,
                    c: c.c,
                    sum: c.sum,
                    carry: c.carry,
                }
                .into(),
            ],
            Project02Component::Project01(Project01Component::Mux(c)) => {
                vec![
                    native::Mux {
                        a0: c.a0,
                        a1: c.a1,
                        sel: c.sel,
                        out: c.out,
                    }
                    .into(),
                ]
            }
            Project02Component::Project01(Project01Component::Mux16(c)) => {
                vec![
                    native::Mux {
                        a0: c.a0,
                        a1: c.a1,
                        sel: c.sel,
                        out: c.out,
                    }
                    .into(),
                ]
            }
            _ => match comp.expand() {
                None => match comp {
                    Project02Component::Project01(p) => crate::project_01::flatten(p)
                        .components
                        .into_iter()
                        .map(|c| {
                            native::Simulational::from(simulator::component::Computational::from(c))
                        })
                        .collect(),
                    _ => panic!("Did not reduce to primitive: {:?}", comp.name()),
                },
                Some(ic) => ic.components.into_iter().flat_map(go).collect(),
            },
        }
    }
    IC {
        name: format!("{} (flat/sim)", chip.name()),
        intf: chip.reflect(),
        components: go(chip.into()),
    }
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

impl Component for HalfAdder {
    type Target = Project01Component;

    /*
    Equivalent to:
      sum = Xor { a = inputs.a, b: inputs.b }
      carry = And {a = inputs.a, b: inputs.b}
    but flattened to use only 5 Nands.
     */
    expand! { |this| {
        // n1 = NAND(a,b) is shared: XOR reuses it, carry = NOT(n1) = NAND(n1,n1)
        n1:    Nand { a: this.a,        b: this.b,        out: Output::new() },
        n2:    Nand { a: this.a,         b: n1.out.into(), out: Output::new() },
        n3:    Nand { a: this.b,         b: n1.out.into(), out: Output::new() },
        sum:   Nand { a: n2.out.into(),  b: n3.out.into(), out: this.sum },
        carry: Nand { a: n1.out.into(),  b: n1.out.into(), out: this.carry },
    }}
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

impl Component for FullAdder {
    type Target = Project01Component;

    /*
    Some sharing of common gates to get down to the minimal 9 gates.
    */
    expand! { |this| {
        // n4 = XOR(a,b); n5 = NAND(c, n4) shared by sum and carry paths
        n1:    Nand { a: this.a,         b: this.b,        out: Output::new() },
        n2:    Nand { a: this.a,         b: n1.out.into(), out: Output::new() },
        n3:    Nand { a: this.b,         b: n1.out.into(), out: Output::new() },
        n4:    Nand { a: n2.out.into(),  b: n3.out.into(), out: Output::new() }, // XOR(a,b)
        n5:    Nand { a: this.c,         b: n4.out.into(), out: Output::new() }, // shared
        n6:    Nand { a: this.c,         b: n5.out.into(), out: Output::new() },
        n7:    Nand { a: n4.out.into(),  b: n5.out.into(), out: Output::new() },
        sum:   Nand { a: n6.out.into(),  b: n7.out.into(), out: this.sum      },
        carry: Nand { a: n1.out.into(),  b: n5.out.into(), out: this.carry    },
    }}
}

/// out = in + 1 (16-bit, overflow ignored)
#[derive(Clone, Reflect, Chip)]
pub struct Inc16 {
    pub a: Input16,
    pub out: Output16,
}

impl Component for Inc16 {
    type Target = Project02Component;

    expand! { |this| {
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
    }}
}

/// out = a + b (16-bit, overflow ignored)
#[derive(Clone, Reflect, Chip)]
pub struct Add16 {
    pub a: Input16,
    pub b: Input16,
    pub out: Output16,
}

impl Component for Add16 {
    type Target = Project02Component;

    expand! { |this| {
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
    }}
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

impl Component for Nand16Way {
    type Target = Project02Component;

    expand! { |this| {
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
    }}
}

/// Returns 1 if all bits of input are 0.
#[derive(Clone, Reflect, Chip)]
pub struct Zero16 {
    pub a: Input16,
    pub out: Output,
}

impl Component for Zero16 {
    type Target = Project02Component;

    expand! { |this| {
        // Negate into a single bus; the simulator makes this parallel.
        not: Not16 { a: this.a, out: Output16::new() },

        // Compare them all at once, as if 16-way fan-in was a thing. The simulator
        // handles this efficiently, too.
        nand_all: Nand16Way { a: not.out.into(), out: Output::new() },

        _f: Not { a: nand_all.out.into(), out: this.out },
    }}
}

/// out = true if the most-significant bit of a is 1 (i.e., input is negative in two's complement).
#[derive(Clone, Reflect, Chip)]
pub struct Neg16 {
    pub a: Input16,
    pub out: Output,
}

impl Component for Neg16 {
    type Target = Project02Component;

    /*
     out = a[15]
    */
    expand! { |this| {
        sign: Buffer { a: this.a.bit(15), out: this.out },
    }}
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

impl Component for ALU {
    type Target = Project02Component;

    expand! { |this| {
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
    }}
}
