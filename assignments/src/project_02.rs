#![allow(unused_variables, dead_code, unused_imports)]

use simulator::{self, Component, Input, Input16, Output, Output16, Reflect};
use simulator::Reflect as _;
use crate::project_01::Project01Component;

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

    fn expand(&self) -> Option<Vec<Project02Component>> {
        todo!()
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
        todo!()
    }
}

// --- Inc16 ---

/// out = in + 1 (16-bit, overflow ignored)
#[derive(Reflect)]
pub struct Inc16 {
    pub in0: Input16,
    pub out: Output16,
}

impl Component for Inc16 {
    type Target = Project02Component;

    fn expand(&self) -> Option<Vec<Project02Component>> {
        todo!()
    }
}

// --- Add16 ---

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
        todo!()
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
        todo!()
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

    fn expand(&self) -> Option<Vec<Project02Component>> {
        todo!()
    }
}

// --- ALU ---

/// Hack ALU: computes one of several functions of x and y selected by control bits.
/// zx: zero the x input
/// nx: negate the x input
/// zy: zero the y input
/// ny: negate the y input
/// f:  if 1, out = x + y; if 0, out = x AND y
/// no: negate the output
/// out: the result
/// zr: 1 if out == 0
/// ng: 1 if out < 0
#[derive(Reflect)]
pub struct Alu {
    pub x:   Input16,
    pub y:   Input16,
    pub zx:  Input,
    pub nx:  Input,
    pub zy:  Input,
    pub ny:  Input,
    pub f:   Input,
    pub no:  Input,
    pub out: Output16,
    pub zr:  Output,
    pub ng:  Output,
}

impl Component for Alu {
    type Target = Project02Component;

    fn expand(&self) -> Option<Vec<Project02Component>> {
        todo!()
    }
}
