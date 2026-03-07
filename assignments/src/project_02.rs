#![allow(unused_variables, dead_code, unused_imports)]

use simulator::{self, Component, Input, Input16, Output, Output16, Reflect};
use simulator::Reflect as _;
use crate::project_01::{Project01Component, Nand, Not, Xor, And, Or};

pub enum Project02Component {
    Project01(Project01Component),
    HalfAdder(HalfAdder),
    FullAdder(FullAdder),
    Inc16(Inc16),
    Add16(Add16),
    Zero16(Zero16),
    Neg16(Neg16),
    Alu(Alu),
}

impl From<Project01Component> for Project02Component { fn from(c: Project01Component) -> Self { Project02Component::Project01(c) } }
impl From<HalfAdder> for Project02Component { fn from(c: HalfAdder) -> Self { Project02Component::HalfAdder(c) } }
impl From<FullAdder> for Project02Component { fn from(c: FullAdder) -> Self { Project02Component::FullAdder(c) } }
impl From<Inc16>     for Project02Component { fn from(c: Inc16)     -> Self { Project02Component::Inc16(c)     } }
impl From<Add16>     for Project02Component { fn from(c: Add16)     -> Self { Project02Component::Add16(c)     } }
impl From<Zero16>    for Project02Component { fn from(c: Zero16)    -> Self { Project02Component::Zero16(c)    } }
impl From<Neg16>     for Project02Component { fn from(c: Neg16)     -> Self { Project02Component::Neg16(c)     } }
impl From<Alu>       for Project02Component { fn from(c: Alu)       -> Self { Project02Component::Alu(c)       } }

impl Component for Project02Component {
    type Target = Project02Component;

    fn expand(&self) -> Option<Vec<Project02Component>> {
        match self {
            Project02Component::Project01(c) => c.expand().map(|v| v.into_iter().map(Into::into).collect()),
            Project02Component::HalfAdder(c) => c.expand(),
            Project02Component::FullAdder(c) => c.expand(),
            Project02Component::Inc16(c)     => c.expand(),
            Project02Component::Add16(c)     => c.expand(),
            Project02Component::Zero16(c)    => c.expand(),
            Project02Component::Neg16(c)     => c.expand(),
            Project02Component::Alu(c)       => c.expand(),
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
            Project02Component::Alu(c)       => c.reflect(),
        }
    }
    fn name(&self) -> &'static str {
        match self {
            Project02Component::Project01(c) => c.name(),
            Project02Component::HalfAdder(c) => c.name(),
            Project02Component::FullAdder(c) => c.name(),
            Project02Component::Inc16(c)     => c.name(),
            Project02Component::Add16(c)     => c.name(),
            Project02Component::Zero16(c)    => c.name(),
            Project02Component::Neg16(c)     => c.name(),
            Project02Component::Alu(c)       => c.name(),
        }
    }
}

/// Recursively expand until only Nands are left.
pub fn flatten<C: Into<Project02Component>>(chip: C) -> Vec<Nand> {
    fn go(comp: Project02Component) -> Vec<Nand> {
        match comp.expand() {
            None => match comp {
                Project02Component::Project01(p) => crate::project_01::flatten(p),
                _ => unreachable!(),
            },
            Some(subs) => subs.into_iter().flat_map(go).collect(),
        }
    }
    go(chip.into())
}

/// sum = 1s-digit of two-bit sum, carry = 2s-digit
#[derive(Reflect)]
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
     */
    fn expand(&self) -> Option<Vec<Project02Component>> {
        let sum   = Xor { a: self.a.clone(), b: self.b.clone(), out: self.sum.clone() };
        let carry = And { a: self.a.clone(), b: self.b.clone(), out: self.carry.clone() };
        Some(vec![
            Project01Component::from(sum).into(),
            Project01Component::from(carry).into(),
        ])
    }
}

/// sum = 1s-digit of three-bit sum, carry = 2s-digit
#[derive(Reflect)]
pub struct FullAdder {
    pub a:     Input,
    pub b:     Input,
    pub c:     Input,
    pub sum:   Output,
    pub carry: Output,
}

impl Component for FullAdder {
    type Target = Project02Component;

    fn expand(&self) -> Option<Vec<Project02Component>> {
        let ha1 = HalfAdder { a: self.a.clone(), b: self.b.clone(), sum: Output::new(), carry: Output::new() };
        let ha2 = HalfAdder { a: ha1.sum.clone().into(), b: self.c.clone(), sum: self.sum.clone(), carry: Output::new() };
        let out_carry = Or { a: ha1.carry.clone().into(), b: ha2.carry.clone().into(), out: self.carry.clone() };
        Some(vec![
            ha1.into(),
            ha2.into(),
            Project01Component::from(out_carry).into(),
        ])
    }
}

// --- Inc16 ---

/// out = in + 1 (16-bit, overflow ignored)
#[derive(Reflect)]
pub struct Inc16 {
    pub a: Input16,
    pub out: Output16,
}

impl Component for Inc16 {
    type Target = Project02Component;

    fn expand(&self) -> Option<Vec<Project02Component>> {
        // bit 0: out[0] = NOT(a[0]); carry = a[0] (the carry-in is implicitly 1)
        let a0   = self.a.bit(0);
        let not0 = Not { a: a0.clone(), out: self.out.bit(0) };
        let mut carry: Input = a0;
        let mut result: Vec<Project02Component> = vec![Project01Component::from(not0).into()];
        for i in 1..16 {
            let ha = HalfAdder { a: self.a.bit(i), b: carry, sum: self.out.bit(i), carry: Output::new() };
            carry = ha.carry.clone().into();
            result.push(ha.into());
        }
        Some(result)
    }
}

/// out = a + b (16-bit, overflow ignored)
#[derive(Reflect)]
pub struct Add16 {
    pub a:   Input16,
    pub b:   Input16,
    pub out: Output16,
}

impl Component for Add16 {
    type Target = Project02Component;

    fn expand(&self) -> Option<Vec<Project02Component>> {
        let ha0 = HalfAdder { a: self.a.bit(0), b: self.b.bit(0), sum: self.out.bit(0), carry: Output::new() };
        let mut carry: Input = ha0.carry.clone().into();
        let mut result: Vec<Project02Component> = vec![ha0.into()];
        for i in 1..16 {
            let fa = FullAdder { a: self.a.bit(i), b: self.b.bit(i), c: carry, sum: self.out.bit(i), carry: Output::new() };
            carry = fa.carry.clone().into();
            result.push(fa.into());
        }
        Some(result)
    }
}

/// Returns 1 if all bits of input are 0.
#[derive(Reflect)]
pub struct Zero16 {
    pub a: Input16,
    pub out: Output,
}

impl Component for Zero16 {
    type Target = Project02Component;

    fn expand(&self) -> Option<Vec<Project02Component>> {
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
        Some(vec![
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
        ])
    }
}

/// out = true if the most-significant bit of in is 1 (i.e., input is negative in two's complement).
#[derive(Reflect)]
pub struct Neg16 {
    pub a: Input16,
    pub out: Output,
}

impl Component for Neg16 {
    type Target = Project02Component;

    /*
      Equivalent to:
      out = a[15]
     */
    fn expand(&self) -> Option<Vec<Project02Component>> {
        // TEMP: pointless gates to express the wiring we need
        let not0 = Not { a: self.a.bit(15), out: Output::new() };
        let not1 = Not { a: not0.out.clone().into(), out: self.out.clone() };
        Some(vec![
            Project01Component::from(not0).into(),
            Project01Component::from(not1).into()])
    }
}

/// Hack ALU: computes one of several functions of x and y selected by control bits.
#[derive(Reflect)]
pub struct Alu {
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
}

impl Component for Alu {
    type Target = Project02Component;

    fn expand(&self) -> Option<Vec<Project02Component>> {
        todo!()
    }
}
