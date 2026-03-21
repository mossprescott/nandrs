#![allow(unused_variables, dead_code, unused_imports)]

use simulator::{self, Component, IC, Input, Input16, Output, Output16, Reflect, AsConst, Chip, expand};
use simulator::Reflect as _;
use simulator::Chip as _;
use simulator::component::{Combinational, FullAdder};
use simulator::nat::N16;
use crate::project_01::{Project01Component, Nand, Const, Buffer, Mux1, Mux16, Not16, And16, Not, Xor, And, Or};

#[derive(Clone)]
pub enum Project02Component {
    Project01(Project01Component),
    // HalfAdder(HalfAdder),
    FullAdder(FullAdder),
    Inc16(Inc16),
    Add16(Add16),
    Zero16(Zero16),
    Neg16(Neg16),
    ALU(ALU),
}

impl<C: Into<Project01Component>> From<C> for Project02Component {
    fn from(c: C) -> Self {
        Project02Component::Project01(c.into())
    }
}
impl From<FullAdder> for Project02Component { fn from(c: FullAdder) -> Self { Project02Component::FullAdder(c) } }
impl From<Inc16>     for Project02Component { fn from(c: Inc16)     -> Self { Project02Component::Inc16(c)     } }
impl From<Add16>     for Project02Component { fn from(c: Add16)     -> Self { Project02Component::Add16(c)     } }
impl From<Zero16>    for Project02Component { fn from(c: Zero16)    -> Self { Project02Component::Zero16(c)    } }
impl From<Neg16>     for Project02Component { fn from(c: Neg16)     -> Self { Project02Component::Neg16(c)     } }
impl From<ALU>       for Project02Component { fn from(c: ALU)       -> Self { Project02Component::ALU(c)       } }

impl Component for Project02Component {
    type Target = Project02Component;

    fn expand(&self) -> Option<IC<Project02Component>> {
        match self {
            Project02Component::Project01(c) => c.expand().map(|ic| IC { name: ic.name, intf: ic.intf, components: ic.components.into_iter().map(Into::into).collect() }),
            Project02Component::FullAdder(_) => None,
            Project02Component::Inc16(c)     => c.expand(),
            Project02Component::Add16(c)     => c.expand(),
            Project02Component::Zero16(c)    => c.expand(),
            Project02Component::Neg16(c)     => c.expand(),
            Project02Component::ALU(c)       => c.expand(),
        }
    }
}

impl Reflect for Project02Component {
    fn reflect(&self) -> simulator::Interface {
        match self {
            Project02Component::Project01(c) => c.reflect(),
            Project02Component::FullAdder(c) => c.reflect(),
            Project02Component::Inc16(c)     => c.reflect(),
            Project02Component::Add16(c)     => c.reflect(),
            Project02Component::Zero16(c)    => c.reflect(),
            Project02Component::Neg16(c)     => c.reflect(),
            Project02Component::ALU(c)       => c.reflect(),
        }
    }
    fn name(&self) -> String {
        match self {
            Project02Component::Project01(c) => c.name(),
            Project02Component::FullAdder(c) => c.name(),
            Project02Component::Inc16(c)     => c.name(),
            Project02Component::Add16(c)     => c.name(),
            Project02Component::Zero16(c)    => c.name(),
            Project02Component::Neg16(c)     => c.name(),
            Project02Component::ALU(c)       => c.name(),
        }
    }
}

impl AsConst for Project02Component {
    fn as_const(&self) -> Option<u64> {
        if let Project02Component::Project01(c) = self { c.as_const() } else { None }
    }
}

/// Recursively expand until only primitives are left.
pub fn flatten<C: Reflect + Into<Project02Component>>(chip: C) -> IC<Combinational<N16>> {
    fn go(comp: Project02Component) -> Vec<Combinational<N16>> {
        match comp.expand() {
            None => match comp {
                Project02Component::Project01(p) => crate::project_01::flatten(p).components,
                Project02Component::FullAdder(c) => vec![Combinational::Adder(c)],
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

/// FullAdder is now provided as a primitive, but it's interesting to implement this separately
/// anyway; this version isn't used by any other components.
///
/// sum = 1s-digit of two-bit sum, carry = 2s-digit
///
/// Future: for pedagocical purposes, define this as HalfAdder here, with reduction to Nands.
#[derive(Clone, Reflect, Chip)]
pub struct MyHalfAdder {
    pub a:     Input,
    pub b:     Input,
    pub sum:   Output,
    pub carry: Output,
}

impl Component for MyHalfAdder {
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

/// FullAdder is now provided as a primitive, but it's interesting to implement separately anyway;
/// this version isn't used by any other components
///
/// sum = 1s-digit of three-bit sum, carry = 2s-digit
///
/// Future: for pedagocical purposes, define this here, with reduction to Nands. Then arrange for it
/// *not* to be expanded when we want to do an efficient simulation.
#[derive(Clone, Reflect, Chip)]
pub struct MyFullAdder {
    pub a:     Input,
    pub b:     Input,
    pub c:     Input,
    pub sum:   Output,
    pub carry: Output,
}

impl Component for MyFullAdder {
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
            zero: Const { value: 0, out: Output16::new() },
            add: FullAdder {
                a: this.a.bit(i),
                b: zero.out.bit(0).into(),
                c: carry,
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
    pub a:   Input16,
    pub b:   Input16,
    pub out: Output16,
}

impl Component for Add16 {
    type Target = Project02Component;

    expand! { |this| {
        // bit 0: half-add (carry-in is 0)
        zero0: Const { value: 0, out: Output16::new() },
        add0: FullAdder {
            a: this.a.bit(0),
            b: this.b.bit(0),
            c: zero0.out.bit(0).into(),
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

/// Returns 1 if all bits of input are 0.
#[derive(Clone, Reflect, Chip)]
pub struct Zero16 {
    pub a: Input16,
    pub out: Output,
}

impl Component for Zero16 {
    type Target = Project02Component;

    expand! { |this| {
        // Level 1: OR adjacent pairs
        or_01:   Or { a: this.a.bit(0),  b: this.a.bit(1),  out: Output::new() },
        or_23:   Or { a: this.a.bit(2),  b: this.a.bit(3),  out: Output::new() },
        or_45:   Or { a: this.a.bit(4),  b: this.a.bit(5),  out: Output::new() },
        or_67:   Or { a: this.a.bit(6),  b: this.a.bit(7),  out: Output::new() },
        or_89:   Or { a: this.a.bit(8),  b: this.a.bit(9),  out: Output::new() },
        or_ab:   Or { a: this.a.bit(10), b: this.a.bit(11), out: Output::new() },
        or_cd:   Or { a: this.a.bit(12), b: this.a.bit(13), out: Output::new() },
        or_ef:   Or { a: this.a.bit(14), b: this.a.bit(15), out: Output::new() },
        // Level 2
        or_0123: Or { a: or_01.out.into(),   b: or_23.out.into(),   out: Output::new() },
        or_4567: Or { a: or_45.out.into(),   b: or_67.out.into(),   out: Output::new() },
        or_89ab: Or { a: or_89.out.into(),   b: or_ab.out.into(),   out: Output::new() },
        or_cdef: Or { a: or_cd.out.into(),   b: or_ef.out.into(),   out: Output::new() },
        // Level 3
        or_lo:   Or { a: or_0123.out.into(), b: or_4567.out.into(), out: Output::new() },
        or_hi:   Or { a: or_89ab.out.into(), b: or_cdef.out.into(), out: Output::new() },
        // Level 4
        or_all:  Or { a: or_lo.out.into(),   b: or_hi.out.into(),   out: Output::new() },
        // Invert: out is 1 iff no bit was set
        _not_all: Not { a: or_all.out.into(), out: this.out },
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
    pub x:   Input16,
    // "Right" input
    pub y:   Input16,
    /// Zero the x input
    pub zx:  Input,
    /// Negate the x input (i.e. "not")
    pub nx:  Input,
    /// Zero the y input
    pub zy:  Input,
    /// Negate the y input (i.e. "not")
    pub ny:  Input,
    /// 0 => x && y; 1 => x + y
    pub f:   Input,
    /// Negate the result (i.e. "not")
    pub no:  Input,

    /// 16-bit result
    pub out: Output16,
    /// Flag: is the result equal to zero (all bits zero)
    pub zr:  Output,
    /// Flag: is the result < 0? (high bit set)
    pub ng:  Output,

    /// Disable: when 1, all outputs are forced to 0. This makes it easier for the simulator to
    /// identify this logic as inactive and avoid spending time evaluating it.
    pub disable: Input,
}

impl Component for ALU {
    type Target = Project02Component;

    expand! { |this| {
        zero_const: Const { value: 0, out: Output16::new() },

        // zx/nx: conditionally zero then negate x
        x1: Mux16 { sel: this.zx, a0: this.x, a1: zero_const.out.into(), out: Output16::new() },
        x2_not: Not16 { a: x1.out.into(), out: Output16::new() },
        x2: Mux16 { sel: this.nx, a0: x1.out.into(), a1: x2_not.out.into(), out: Output16::new() },

        // zy/ny: conditionally zero then negate y
        y1: Mux16 { sel: this.zy, a0: this.y, a1: zero_const.out.into(), out: Output16::new() },
        y2_not: Not16 { a: y1.out.into(), out: Output16::new() },
        y2: Mux16 { sel: this.ny, a0: y1.out.into(), a1: y2_not.out.into(), out: Output16::new() },

        and: And16 { a: x2.out.into(), b: y2.out.into(), out: Output16::new() },

        // Gate Add16 inputs: only active when f=1 AND !disable. Mux16 gives Add16
        // its own copy of the inputs, breaking the sharing with And16 so the nesting
        // algorithm can move all Add16 nands into a mux branch.
        not_disable: Not { a: this.disable, out: Output::new() },
        add_active: And { a: this.f, b: not_disable.out.into(), out: Output::new() },
        add_x: Mux16 { sel: add_active.out.into(), a0: zero_const.out.into(), a1: x2.out.into(), out: Output16::new() },
        add_y: Mux16 { sel: add_active.out.into(), a0: zero_const.out.into(), a1: y2.out.into(), out: Output16::new() },
        add: Add16 { a: add_x.out.into(), b: add_y.out.into(), out: Output16::new() },

        result: Mux16 { sel: this.f, a0: and.out.into(), a1: add.out.into(), out: Output16::new() },
        rn: Not16 { a: result.out.into(), out: Output16::new() },
        raw_out: Mux16 { sel: this.no, a0: result.out.into(), a1: rn.out.into(), out: Output16::new() },

        // Compute zr from raw_out (before the disable gate) so it can be nested
        // into the disable mux branch along with the rest of the ALU chain.
        raw_zr: Zero16 { a: raw_out.out.into(), out: Output::new() },

        // Gate output and zr with disable.  When disabled: out=0, zr=1.
        const_one: Const { value: 1, out: Output::new() },
        out_gate: Mux16 { sel: this.disable, a0: raw_out.out.into(), a1: zero_const.out.into(), out: this.out },
        zr_gate: Mux1 { sel: this.disable, a0: raw_zr.out.into(), a1: const_one.out.bit(0).into(), out: this.zr },

        // ng reads from the gated output; when disabled out=0 so ng=0 (correct).
        // Neg16 is 0 nands (just a buffer) so there's nothing to skip.
        rneg: Neg16 { a: this.out.into(), out: this.ng },
   }}
}
