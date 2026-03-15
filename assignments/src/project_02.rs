#![allow(unused_variables, dead_code, unused_imports)]

use simulator::{self, Component, IC, Input, Input16, Output, Output16, Reflect, AsConst, Chip};
use simulator::Reflect as _;
use simulator::Chip as _;
use simulator::component::Combinational;
use simulator::nat::N16;
use crate::project_01::{Project01Component, Nand, Const, Buffer, Mux16, Not16, And16, Not, Xor, And, Or};

pub enum Project02Component {
    Project01(Project01Component),
    HalfAdder(HalfAdder),
    FullAdder(FullAdder),
    Inc16(Inc16),
    Add16(Add16),
    Zero16(Zero16),
    Neg16(Neg16),
    ALU(ALU),
}

impl From<Project01Component> for Project02Component { fn from(c: Project01Component) -> Self { Project02Component::Project01(c) } }
impl From<HalfAdder> for Project02Component { fn from(c: HalfAdder) -> Self { Project02Component::HalfAdder(c) } }
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
            Project02Component::HalfAdder(c) => c.expand(),
            Project02Component::FullAdder(c) => c.expand(),
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
            Project02Component::HalfAdder(c) => c.reflect(),
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
            Project02Component::HalfAdder(c) => c.name(),
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

/// sum = 1s-digit of two-bit sum, carry = 2s-digit
#[derive(Reflect, Chip)]
pub struct HalfAdder {
    pub a:     Input,
    pub b:     Input,
    pub sum:   Output,
    pub carry: Output,
}

impl Component for HalfAdder {
    type Target = Project02Component;

    /*
    Equivalent to:
      sum = Xor { a = inputs.a, b: inputs.b }
      carry = And {a = inputs.a, b: inputs.b}
    but flattened to use only 5 Nands.
     */
    fn expand(&self) -> Option<IC<Project02Component>> {
        // n1 = NAND(a,b) is shared: XOR reuses it, carry = NOT(n1) = NAND(n1,n1)
        let n1    = Nand { a: self.a.clone(),        b: self.b.clone(),        out: Output::new() };
        let n2    = Nand { a: self.a.clone(),         b: n1.out.clone().into(), out: Output::new() };
        let n3    = Nand { a: self.b.clone(),         b: n1.out.clone().into(), out: Output::new() };
        let sum   = Nand { a: n2.out.clone().into(),  b: n3.out.clone().into(), out: self.sum.clone() };
        let carry = Nand { a: n1.out.clone().into(),  b: n1.out.clone().into(), out: self.carry.clone() };
        Some(IC { name: self.name().to_string(), intf: self.reflect(), components: vec![
            Project01Component::from(n1).into(),
            Project01Component::from(n2).into(),
            Project01Component::from(n3).into(),
            Project01Component::from(sum).into(),
            Project01Component::from(carry).into(),
        ]})
    }
}

/// sum = 1s-digit of three-bit sum, carry = 2s-digit
#[derive(Reflect, Chip)]
pub struct FullAdder {
    pub a:     Input,
    pub b:     Input,
    pub c:     Input,
    pub sum:   Output,
    pub carry: Output,
}

impl Component for FullAdder {
    type Target = Project02Component;

    /*
     Some sharing of common gates to get down to the minimal 9 gates.
     */
    fn expand(&self) -> Option<IC<Project02Component>> {
        // n4 = XOR(a,b); n5 = NAND(c, n4) shared by sum and carry paths
        let n1    = Nand { a: self.a.clone(),        b: self.b.clone(),        out: Output::new() };
        let n2    = Nand { a: self.a.clone(),         b: n1.out.clone().into(), out: Output::new() };
        let n3    = Nand { a: self.b.clone(),         b: n1.out.clone().into(), out: Output::new() };
        let n4    = Nand { a: n2.out.clone().into(),  b: n3.out.clone().into(), out: Output::new() }; // XOR(a,b)
        let n5    = Nand { a: self.c.clone(),         b: n4.out.clone().into(), out: Output::new() }; // shared
        let n6    = Nand { a: self.c.clone(),         b: n5.out.clone().into(), out: Output::new() };
        let n7    = Nand { a: n4.out.clone().into(),  b: n5.out.clone().into(), out: Output::new() };
        let sum   = Nand { a: n6.out.clone().into(),  b: n7.out.clone().into(), out: self.sum.clone() };
        let carry = Nand { a: n1.out.clone().into(),  b: n5.out.clone().into(), out: self.carry.clone() };
        Some(IC { name: self.name().to_string(), intf: self.reflect(), components: vec![
            Project01Component::from(n1).into(),
            Project01Component::from(n2).into(),
            Project01Component::from(n3).into(),
            Project01Component::from(n4).into(),
            Project01Component::from(n5).into(),
            Project01Component::from(n6).into(),
            Project01Component::from(n7).into(),
            Project01Component::from(sum).into(),
            Project01Component::from(carry).into(),
        ]})
    }
}

/// out = in + 1 (16-bit, overflow ignored)
#[derive(Reflect, Chip)]
pub struct Inc16 {
    pub a: Input16,
    pub out: Output16,
}

impl Component for Inc16 {
    type Target = Project02Component;

    fn expand(&self) -> Option<IC<Project02Component>> {
        // bit 0: out[0] = NOT(a[0]); carry = a[0] (the carry-in is implicitly 1)
        let a0   = self.a.bit(0);
        let not0 = Not { a: a0.clone(), out: self.out.bit(0) };
        let mut carry: Input = a0;
        let mut components: Vec<Project02Component> = vec![Project01Component::from(not0).into()];
        for i in 1..16 {
            let ha = HalfAdder { a: self.a.bit(i), b: carry, sum: self.out.bit(i), carry: Output::new() };
            carry = ha.carry.clone().into();
            components.push(ha.into());
        }
        Some(IC { name: self.name().to_string(), intf: self.reflect(), components })
    }
}

/// out = a + b (16-bit, overflow ignored)
#[derive(Reflect, Chip)]
pub struct Add16 {
    pub a:   Input16,
    pub b:   Input16,
    pub out: Output16,
}

impl Component for Add16 {
    type Target = Project02Component;

    fn expand(&self) -> Option<IC<Project02Component>> {
        let ha0 = HalfAdder { a: self.a.bit(0), b: self.b.bit(0), sum: self.out.bit(0), carry: Output::new() };
        let mut carry: Input = ha0.carry.clone().into();
        let mut components: Vec<Project02Component> = vec![ha0.into()];
        for i in 1..16 {
            let fa = FullAdder { a: self.a.bit(i), b: self.b.bit(i), c: carry, sum: self.out.bit(i), carry: Output::new() };
            carry = fa.carry.clone().into();
            components.push(fa.into());
        }
        Some(IC { name: self.name().to_string(), intf: self.reflect(), components })
    }
}

/// Returns 1 if all bits of input are 0.
#[derive(Reflect, Chip)]
pub struct Zero16 {
    pub a: Input16,
    pub out: Output,
}

impl Component for Zero16 {
    type Target = Project02Component;

    fn expand(&self) -> Option<IC<Project02Component>> {
        // Level 1: OR adjacent pairs
        let or_01   = Or { a: self.a.bit(0),               b: self.a.bit(1),               out: Output::new() };
        let or_23   = Or { a: self.a.bit(2),               b: self.a.bit(3),               out: Output::new() };
        let or_45   = Or { a: self.a.bit(4),               b: self.a.bit(5),               out: Output::new() };
        let or_67   = Or { a: self.a.bit(6),               b: self.a.bit(7),               out: Output::new() };
        let or_89   = Or { a: self.a.bit(8),               b: self.a.bit(9),               out: Output::new() };
        let or_ab   = Or { a: self.a.bit(10),              b: self.a.bit(11),              out: Output::new() };
        let or_cd   = Or { a: self.a.bit(12),              b: self.a.bit(13),              out: Output::new() };
        let or_ef   = Or { a: self.a.bit(14),              b: self.a.bit(15),              out: Output::new() };
        // Level 2
        let or_0123 = Or { a: or_01.out.clone().into(),    b: or_23.out.clone().into(),    out: Output::new() };
        let or_4567 = Or { a: or_45.out.clone().into(),    b: or_67.out.clone().into(),    out: Output::new() };
        let or_89ab = Or { a: or_89.out.clone().into(),    b: or_ab.out.clone().into(),    out: Output::new() };
        let or_cdef = Or { a: or_cd.out.clone().into(),    b: or_ef.out.clone().into(),    out: Output::new() };
        // Level 3
        let or_lo   = Or { a: or_0123.out.clone().into(),  b: or_4567.out.clone().into(),  out: Output::new() };
        let or_hi   = Or { a: or_89ab.out.clone().into(),  b: or_cdef.out.clone().into(),  out: Output::new() };
        // Level 4
        let or_all  = Or { a: or_lo.out.clone().into(),    b: or_hi.out.clone().into(),    out: Output::new() };
        // Invert: out is 1 iff no bit was set
        let not_all = Not { a: or_all.out.clone().into(), out: self.out.clone() };
        Some(IC { name: self.name().to_string(), intf: self.reflect(), components: vec![
            Project01Component::from(or_01).into(),
            Project01Component::from(or_23).into(),
            Project01Component::from(or_45).into(),
            Project01Component::from(or_67).into(),
            Project01Component::from(or_89).into(),
            Project01Component::from(or_ab).into(),
            Project01Component::from(or_cd).into(),
            Project01Component::from(or_ef).into(),
            Project01Component::from(or_0123).into(),
            Project01Component::from(or_4567).into(),
            Project01Component::from(or_89ab).into(),
            Project01Component::from(or_cdef).into(),
            Project01Component::from(or_lo).into(),
            Project01Component::from(or_hi).into(),
            Project01Component::from(or_all).into(),
            Project01Component::from(not_all).into(),
        ]})
    }
}

/// out = true if the most-significant bit of a is 1 (i.e., input is negative in two's complement).
#[derive(Reflect, Chip)]
pub struct Neg16 {
    pub a: Input16,
    pub out: Output,
}

impl Component for Neg16 {
    type Target = Project02Component;

    /*
      out = a[15]
     */
    fn expand(&self) -> Option<IC<Project02Component>> {
        let buffer = Buffer { a: self.a.bit(15), out: self.out.clone() };
        Some(IC {
            name: self.name().to_string(),
            intf: self.reflect(),
            components: vec![
                Project01Component::from(buffer).into(),
            ]
        })
    }
}

/// Hack ALU: computes one of several functions of x and y selected by control bits.
#[derive(Reflect, Chip)]
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

    fn expand(&self) -> Option<IC<Project02Component>> {
        let zero_const = Const { value: 0, out: Output16::new() };

        // zx/nx: conditionally zero then negate x
        let x1 = Mux16 { sel: self.zx.clone(), a0: self.x.clone(), a1: zero_const.out.clone().into(), out: Output16::new() };
        let x2_not = Not16 { a: x1.out.clone().into(), out: Output16::new() };
        let x2 = Mux16 { sel: self.nx.clone(), a0: x1.out.clone().into(), a1: x2_not.out.clone().into(), out: Output16::new() };

        // zy/ny: conditionally zero then negate y
        let y1 = Mux16 { sel: self.zy.clone(), a0: self.y.clone(), a1: zero_const.out.clone().into(), out: Output16::new() };
        let y2_not = Not16 { a: y1.out.clone().into(), out: Output16::new() };
        let y2 = Mux16 { sel: self.ny.clone(), a0: y1.out.clone().into(), a1: y2_not.out.clone().into(), out: Output16::new() };

        let and = And16 { a: x2.out.clone().into(), b: y2.out.clone().into(), out: Output16::new() };

        // Gate Add16 inputs: only active when f=1 AND !disable. Mux16 gives Add16
        // its own copy of the inputs, breaking the sharing with And16 so the nesting
        // algorithm can move all Add16 nands into a mux branch.
        let not_disable = Not { a: self.disable.clone(), out: Output::new() };
        let add_active = And { a: self.f.clone(), b: not_disable.out.clone().into(), out: Output::new() };
        let add_x = Mux16 { sel: add_active.out.clone().into(), a0: zero_const.out.clone().into(), a1: x2.out.clone().into(), out: Output16::new() };
        let add_y = Mux16 { sel: add_active.out.clone().into(), a0: zero_const.out.clone().into(), a1: y2.out.clone().into(), out: Output16::new() };
        let add = Add16 { a: add_x.out.clone().into(), b: add_y.out.clone().into(), out: Output16::new() };

        let result = Mux16 { sel: self.f.clone(), a0: and.out.clone().into(), a1: add.out.clone().into(), out: Output16::new() };
        let rn = Not16 { a: result.out.clone().into(), out: Output16::new() };
        let raw_out = Mux16 { sel: self.no.clone(), a0: result.out.clone().into(), a1: rn.out.clone().into(), out: Output16::new() };

        // Gate output with disable (when disable=1, force to 0; raw_out becomes single-consumer
        // so the entire ALU compute chain can be nested into the mux branch)
        let out_gate = Mux16 { sel: self.disable.clone(), a0: raw_out.out.clone().into(), a1: zero_const.out.clone().into(), out: self.out.clone() };

        // Flags read from gated output: when disabled, zr=1 and ng=0, which is harmless
        // because the CPU gates all jump/write decisions with is_c.
        let rz = Zero16 { a: self.out.clone().into(), out: self.zr.clone() };
        let rneg = Neg16 { a: self.out.clone().into(), out: self.ng.clone() };

        Some(IC { name: self.name().to_string(), intf: self.reflect(), components: vec![
            Project01Component::from(zero_const).into(),
            Project01Component::from(x1).into(),
            Project01Component::from(x2_not).into(),
            Project01Component::from(x2).into(),
            Project01Component::from(y1).into(),
            Project01Component::from(y2_not).into(),
            Project01Component::from(y2).into(),
            Project01Component::from(and).into(),
            Project01Component::from(not_disable).into(),
            Project01Component::from(add_active).into(),
            Project01Component::from(add_x).into(),
            Project01Component::from(add_y).into(),
            add.into(),
            Project01Component::from(result).into(),
            Project01Component::from(rn).into(),
            Project01Component::from(raw_out).into(),
            Project01Component::from(out_gate).into(),
            rz.into(),
            rneg.into(),
        ]})
   }
}
