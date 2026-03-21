#![allow(unused_variables, dead_code, unused_imports)]

use simulator::{self, Component, IC, Input, Input16, Output, Output16, Reflect, Chip, expand};
use simulator::component::Combinational;
use simulator::nat::{N1, N16};
use simulator::AsConst;
use simulator::Reflect as _;
use simulator::Chip as _;
use std::collections::HashMap;

// Re-export since the other components here parallel Nand:
pub use simulator::component::{Nand, Const, Buffer, Mux1, Mux16};

/// Components implemented in this project: simple, logical components for 1 and 16 bits.
#[derive(Clone)]
pub enum Project01Component {
    // primitive:
    Nand(Nand),
    Buffer(Buffer),
    Const(Const),
    Mux1(Mux1),
    Mux16(Mux16),
    // non-primitive:
    Not(Not),
    And(And),
    Or(Or),
    Xor(Xor),
    Dmux(Dmux),
    Not16(Not16),
    And16(And16),
    // Or16(Or16),
}

// primitive:
impl From<Nand>   for Project01Component { fn from(c: Nand)   -> Self { Project01Component::Nand(c)   } }
impl From<Buffer> for Project01Component { fn from(c: Buffer) -> Self { Project01Component::Buffer(c) } }
impl From<Const>  for Project01Component { fn from(c: Const)  -> Self { Project01Component::Const(c)  } }
impl From<Mux1>   for Project01Component { fn from(c: Mux1)   -> Self { Project01Component::Mux1(c)   } }
impl From<Mux16>  for Project01Component { fn from(c: Mux16)  -> Self { Project01Component::Mux16(c)  } }
// non-primitive:
impl From<Not>   for Project01Component { fn from(c: Not)   -> Self { Project01Component::Not(c)   } }
impl From<And>   for Project01Component { fn from(c: And)   -> Self { Project01Component::And(c)   } }
impl From<Or>    for Project01Component { fn from(c: Or)    -> Self { Project01Component::Or(c)    } }
impl From<Xor>   for Project01Component { fn from(c: Xor)   -> Self { Project01Component::Xor(c)   } }
impl From<Dmux>  for Project01Component { fn from(c: Dmux)  -> Self { Project01Component::Dmux(c)  } }
impl From<Not16> for Project01Component { fn from(c: Not16) -> Self { Project01Component::Not16(c) } }
impl From<And16> for Project01Component { fn from(c: And16) -> Self { Project01Component::And16(c) } }
// impl From<Or16>  for Project01Component { fn from(c: Or16)  -> Self { Project01Component::Or16(c)  } }

impl Component for Project01Component {
    type Target = Project01Component;

    fn expand(&self) -> Option<IC<Project01Component>> {
        match self {
            // primitive:
            Project01Component::Nand(c)  => None,
            Project01Component::Buffer(c) => None,
            Project01Component::Const(c) => None,
            Project01Component::Mux1(c)  => None,
            Project01Component::Mux16(c) => None,
            // non-primitive:
            Project01Component::Not(c)   => c.expand(),
            Project01Component::And(c)   => c.expand(),
            Project01Component::Or(c)    => c.expand(),
            Project01Component::Xor(c)   => c.expand(),
            Project01Component::Dmux(c)  => c.expand(),
            Project01Component::Not16(c) => c.expand(),
            Project01Component::And16(c) => c.expand(),
        }
    }
}
impl Reflect for Project01Component {
    fn reflect(&self) -> simulator::Interface {
        match self {
            // primitive:
            Project01Component::Nand(c)  => c.reflect(),
            Project01Component::Buffer(c) => c.reflect(),
            Project01Component::Const(c) => c.reflect(),
            Project01Component::Mux1(c)  => c.reflect(),
            Project01Component::Mux16(c) => c.reflect(),
            // non-primitive:
            Project01Component::Not(c)   => c.reflect(),
            Project01Component::And(c)   => c.reflect(),
            Project01Component::Or(c)    => c.reflect(),
            Project01Component::Xor(c)   => c.reflect(),
            Project01Component::Dmux(c)  => c.reflect(),
            Project01Component::Not16(c) => c.reflect(),
            Project01Component::And16(c) => c.reflect(),
        }
    }
    fn name(&self) -> String {
        match self {
            // primitive:
            Project01Component::Nand(c)  => c.name(),
            Project01Component::Buffer(c) => c.name(),
            Project01Component::Const(c) => c.name(),
            Project01Component::Mux1(c)  => c.name(),
            Project01Component::Mux16(c) => c.name(),
            // non-primitive:
            Project01Component::Not(c)   => c.name(),
            Project01Component::And(c)   => c.name(),
            Project01Component::Or(c)    => c.name(),
            Project01Component::Xor(c)   => c.name(),
            Project01Component::Dmux(c)  => c.name(),
            Project01Component::Not16(c) => c.name(),
            Project01Component::And16(c) => c.name(),
        }
    }
}

impl AsConst for Project01Component {
    fn as_const(&self) -> Option<u64> {
        if let Project01Component::Const(c) = self { c.as_const() } else { None }
    }
}

/// Recursively expand() until only primitives are left.
pub fn flatten<C: Reflect + Into<Project01Component>>(chip: C) -> IC<Combinational<N16>> {
    fn go(comp: Project01Component) -> Vec<Combinational<N16>> {
        match comp.expand() {
            None => match comp {
                Project01Component::Nand(c) => vec![c.into()],
                Project01Component::Const(c) => vec![c.into()],
                Project01Component::Buffer(c) => vec![c.into()],
                Project01Component::Mux1(c) => vec![Combinational::Mux1(c)],
                Project01Component::Mux16(c) => vec![Combinational::Mux(c)],
                _ => panic!("Did not reduce to primitive: {:?}", comp.name()),
            },
            Some(ic) => ic.components.into_iter().flat_map(go).collect(),
        }
    }
    IC { name: format!("{} (flat)", chip.name()),
        intf: chip.reflect(),
        components: go(chip.into()),
    }
}


/// Inverts its input.
#[derive(Clone, Reflect, Chip)]
pub struct Not {
    pub a: Input,
    pub out: Output,
}
impl Component for Not {
    type Target = Project01Component;

    expand! { |this| {
        nand: Nand {
            a: this.a,
            b: this.a,  // also the "a" input
            out: this.out,
        }
    }}
}

/// True only when both inputs are true.
#[derive(Clone, Reflect, Chip)]
pub struct And {
    pub a: Input,
    pub b: Input,
    pub out: Output,
}
impl Component for And {
    type Target = Project01Component;

    expand! { |this| {
        nand: Nand {
            a: this.a,
            b: this.b,
            out: Output::new(),
        },
        not: Not {
            a: nand.out.into(),
            out: this.out,
        },
    }}
}

/// True when at least one input is true.
#[derive(Clone, Reflect, Chip)]
pub struct Or {
    pub a: Input,
    pub b: Input,
    pub out: Output,
}
impl Component for Or {
    type Target = Project01Component;

    expand! { |this| {
        not_a: Not { a: this.a, out: Output::new() },
        not_b: Not { a: this.b, out: Output::new() },
        nand: Nand { a: not_a.out.into(), b: not_b.out.into(), out: this.out }
    }}
}

/// True when inputs differ.
#[derive(Clone, Reflect, Chip)]
pub struct Xor {
    pub a: Input,
    pub b: Input,
    pub out: Output,
}
impl Component for Xor {
    type Target = Project01Component;

    expand! { |this| {
        n1:  Nand { a: this.a,        b: this.b,        out: Output::new() },
        n2:  Nand { a: this.a,        b: n1.out.into(), out: Output::new() },
        n3:  Nand { a: this.b,        b: n1.out.into(), out: Output::new() },
        out: Nand { a: n2.out.into(), b: n3.out.into(), out: this.out },
    }}
}

/// Mux is primitive and general; lets give it a name for when we only need one bit.
pub type Mux = simulator::component::Mux<N1>;

/// Mux is now provided as a primitive (for arbitrary bits wide), but it's interesting to
/// implement separately anyway; this version isn't used by any other components
///
/// Passes a0 through when sel is 0, a1 when sel is 1.
#[derive(Clone, Reflect, Chip)]
pub struct MyMux {
    pub a0: Input,
    pub a1: Input,
    pub sel: Input,
    pub out: Output,
}
impl Component for MyMux {
    type Target = Project01Component;

    expand! { |this| {
        not_sel: Not  { a: this.sel,            out: Output::new() },
        nand0:   Nand { a: not_sel.out.into(),  b: this.a0,          out: Output::new() },
        nand1:   Nand { a: this.sel,            b: this.a1,          out: Output::new() },
        out:     Nand { a: nand0.out.into(),    b: nand1.out.into(), out: this.out },
    }}
}

/// Routes input to a when sel is 0, or b when sel is 1; the unused output is zero.
#[derive(Clone, Reflect, Chip)]
pub struct Dmux {
    pub input: Input,
    pub sel: Input,
    pub a: Output,
    pub b: Output,
}
impl Component for Dmux {
    type Target = Project01Component;

    expand! { |this| {
        not_sel: Not { a: this.sel,   out: Output::new() },

        and_a:   And { a: this.input, b: not_sel.out.into(), out: this.a },
        and_b:   And { a: this.input, b: this.sel,           out: this.b },
    }}
}

/// Inverts each bit of a 16-bit input.
#[derive(Clone, Reflect, Chip)]
pub struct Not16 {
    pub a: Input16,
    pub out: Output16,
}
impl Component for Not16 {
    type Target = Project01Component;

    expand! { |this| {
        for i in 0..16 {
            _not: Not { a: this.a.bit(i), out: this.out.bit(i) }
        }
    }}
}

/// Bitwise `And` across two 16-bit inputs.
#[derive(Clone, Reflect, Chip)]
pub struct And16 {
    pub a: Input16,
    pub b: Input16,
    pub out: Output16,
}
impl Component for And16 {
    type Target = Project01Component;

    expand! { |this| {
        for i in 0..16 {
            _and: And { a: this.a.bit(i), b: this.b.bit(i), out: this.out.bit(i) }
        }
    }}
}

// /// Selects between two 16-bit inputs bit-by-bit, using a single sel bit.
// #[derive(Clone, Reflect, Chip)]
// pub struct Mux16 {
//     pub a0: Input16,
//     pub a1: Input16,
//     pub sel: Input,
//     pub out: Output16,
// }
// impl Component for Mux16 {
//     type Target = Project01Component;

//     /*
//       let not_sel = Not { a: sel }
//       for i in 0..16:
//         let nand0      = Nand { a: not_sel.out, b: a0[i]    }
//         let nand1      = Nand { a: sel,         b: a1[i]    }
//         outputs.out[i] = Nand { a: nand0.out,   b: nand1.out }
//      */
//     fn expand(&self) -> Option<IC<Project01Component>> {
//         let not_sel = Not { a: self.sel.clone(), out: Output::new() };
//         let not_sel_out: Input = not_sel.out.clone().into();

//         let mut components = vec![not_sel.into()];
//         components.extend((0..16).flat_map(|i| {
//             let nand0 = Nand { a: not_sel_out.clone(),       b: self.a0.bit(i),        out: Output::new() };
//             let nand1 = Nand { a: self.sel.clone(),           b: self.a1.bit(i),        out: Output::new() };
//             let out   = Nand { a: nand0.out.clone().into(),  b: nand1.out.clone().into(), out: self.out.bit(i) };
//             vec![nand0.into(), nand1.into(), out.into()]
//         }).collect::<Vec<_>>());
//         Some(IC { name: self.name().to_string(), intf: self.reflect(), components })
//     }
// }


// These are needed for RAMs, maybe? Nevermind that stuff.
//   mux4way16
//   mux8way16
//   dmux4way
//   dmux8way
