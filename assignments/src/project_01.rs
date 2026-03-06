#![allow(unused_variables, dead_code, unused_imports)]

use simulator::{self, Component, Input, Input16, Output, Output16, Reflect};
use simulator::Reflect as _; // ensure the derive macro is in scope
use std::collections::HashMap;

/// The single primitive: true if either input is false.
#[derive(Reflect)]
pub struct Nand {
    pub a: Input,
    pub b: Input,
    pub out: Output,
}
/// Nothing to expand; Nand is Nand.
impl Component for Nand {
    type Target = Nand;

    fn expand(&self) -> Option<Vec<Nand>> {
       Option::None
    }
}

/// Components implemented in this project: simple, logical components for 1 and 16 bits.
pub enum Project01Component {
    Nand(Nand),
    Not(Not),
    And(And),
    Or(Or),
    Xor(Xor),
    Mux(Mux),
    Dmux(Dmux),
    Not16(Not16),
    And16(And16),
    // Or16(Or16),
    Mux16(Mux16),
}

impl From<Nand>  for Project01Component { fn from(c: Nand)  -> Self { Project01Component::Nand(c)  } }
impl From<Not>   for Project01Component { fn from(c: Not)   -> Self { Project01Component::Not(c)   } }
impl From<And>   for Project01Component { fn from(c: And)   -> Self { Project01Component::And(c)   } }
impl From<Or>    for Project01Component { fn from(c: Or)    -> Self { Project01Component::Or(c)    } }
impl From<Xor>   for Project01Component { fn from(c: Xor)   -> Self { Project01Component::Xor(c)   } }
impl From<Mux>   for Project01Component { fn from(c: Mux)   -> Self { Project01Component::Mux(c)   } }
impl From<Dmux>  for Project01Component { fn from(c: Dmux)  -> Self { Project01Component::Dmux(c)  } }
impl From<Not16> for Project01Component { fn from(c: Not16) -> Self { Project01Component::Not16(c) } }
impl From<And16> for Project01Component { fn from(c: And16) -> Self { Project01Component::And16(c) } }
// impl From<Or16>  for Project01Component { fn from(c: Or16)  -> Self { Project01Component::Or16(c)  } }
impl From<Mux16> for Project01Component { fn from(c: Mux16) -> Self { Project01Component::Mux16(c) } }

impl Component for Project01Component {
    type Target = Project01Component;

    fn expand(&self) -> Option<Vec<Project01Component>> {
        match self {
            Project01Component::Nand(c)  => c.expand().map(|v| v.into_iter().map(Into::into).collect()),
            Project01Component::Not(c)   => c.expand(),
            Project01Component::And(c)   => c.expand(),
            Project01Component::Or(c)    => c.expand(),
            Project01Component::Xor(c)   => c.expand(),
            Project01Component::Mux(c)   => c.expand(),
            Project01Component::Dmux(c)  => c.expand(),
            Project01Component::Not16(c) => c.expand(),
            Project01Component::And16(c) => c.expand(),
            Project01Component::Mux16(c) => c.expand(),
        }
    }
}
impl Reflect for Project01Component {
    fn reflect(&self) -> simulator::Interface {
        match self {
            Project01Component::Nand(c)  => c.reflect(),
            Project01Component::Not(c)   => c.reflect(),
            Project01Component::And(c)   => c.reflect(),
            Project01Component::Or(c)    => c.reflect(),
            Project01Component::Xor(c)   => c.reflect(),
            Project01Component::Mux(c)   => c.reflect(),
            Project01Component::Dmux(c)  => c.reflect(),
            Project01Component::Not16(c) => c.reflect(),
            Project01Component::And16(c) => c.reflect(),
            Project01Component::Mux16(c) => c.reflect(),
        }
    }
    fn name(&self) -> &'static str {
        match self {
            Project01Component::Nand(c)  => c.name(),
            Project01Component::Not(c)   => c.name(),
            Project01Component::And(c)   => c.name(),
            Project01Component::Or(c)    => c.name(),
            Project01Component::Xor(c)   => c.name(),
            Project01Component::Mux(c)   => c.name(),
            Project01Component::Dmux(c)  => c.name(),
            Project01Component::Not16(c) => c.name(),
            Project01Component::And16(c) => c.name(),
            Project01Component::Mux16(c) => c.name(),
        }
    }
}

/// Recursively expand() until only Nands are left.
pub fn flatten<C: Into<Project01Component>>(chip: C) -> Vec<Nand> {
    fn go(comp: Project01Component) -> Vec<Nand> {
        match comp.expand() {
            None => match comp {
                Project01Component::Nand(nand) => vec![nand],
                _ => unreachable!(),
            },
            Some(subs) => subs.into_iter().flat_map(go).collect(),
        }
    }
    go(chip.into())
}


/// Inverts its input.
#[derive(Reflect)]
pub struct Not {
    pub a: Input,
    pub out: Output,
}
impl Component for Not {
    type Target = Project01Component;

    /*
      let nand = Nand { a: inputs.a, b: inputs.b }
      outputs.out = nand.out
     */
    fn expand(&self) -> Option<Vec<Project01Component>> {
        let nand = Nand {
            a: self.a.clone(),
            b: self.a.clone(),
            out: self.out.clone(),
        };
        Option::Some(vec![nand.into()])
    }
}

/// True only when both inputs are true.
#[derive(Reflect)]
pub struct And {
    pub a: Input,
    pub b: Input,
    pub out: Output,
}
impl Component for And {
    type Target = Project01Component;

   /*
      let nand = Nand { a: inputs.a, b: inputs.b }
      let not = Not { a: nand.out }
      outputs.out = not.out
     */
    fn expand(&self) -> Option<Vec<Project01Component>> {
        let nand = Nand { a: self.a.clone(), b: self.b.clone(), out: Output::new() };
        let not  = Not  { a: nand.out.clone().into(),            out: self.out.clone() };
        Option::Some(vec![nand.into(), not.into()])
    }
}

/// True when at least one input is true.
#[derive(Reflect)]
pub struct Or {
    pub a: Input,
    pub b: Input,
    pub out: Output,
}
impl Component for Or {
    type Target = Project01Component;

    /*
      let not_a = Not { a: inputs.a }
      let not_b = Not { a: inputs.b }
      let nand = Nand { a: not_a.out, b: not_b.out}
      outputs.out = nand.out
     */
    fn expand(&self) -> Option<Vec<Project01Component>> {
        let not_a = Not  { a: self.a.clone(), out: Output::new() };
        let not_b = Not  { a: self.b.clone(), out: Output::new() };
        let nand  = Nand { a: not_a.out.clone().into(), b: not_b.out.clone().into(), out: self.out.clone() };
        Some(vec![not_a.into(), not_b.into(), nand.into()])
    }
}

/// True when inputs differ.
#[derive(Reflect)]
pub struct Xor {
    pub a: Input,
    pub b: Input,
    pub out: Output,
}
impl Component for Xor {
    type Target = Project01Component;

    /*
      let n1  = Nand { a: a, b: b     }
      let n2  = Nand { a: a, b: n1.out }
      let n3  = Nand { a: b, b: n1.out }
      outputs.out = Nand { a: n2.out, b: n3.out }
     */
    fn expand(&self) -> Option<Vec<Project01Component>> {
        let n1  = Nand { a: self.a.clone(),        b: self.b.clone(),        out: Output::new() };
        let n2  = Nand { a: self.a.clone(),        b: n1.out.clone().into(), out: Output::new() };
        let n3  = Nand { a: self.b.clone(),        b: n1.out.clone().into(), out: Output::new() };
        let out = Nand { a: n2.out.clone().into(), b: n3.out.clone().into(), out: self.out.clone() };
        Some(vec![n1.into(), n2.into(), n3.into(), out.into()])
    }
}

/// Passes a0 through when sel is 0, a1 when sel is 1.
#[derive(Reflect)]
pub struct Mux {
    pub a0: Input,
    pub a1: Input,
    pub sel: Input,
    pub out: Output,
}
impl Component for Mux {
    type Target = Project01Component;

    /*
      let not_sel = Not { a: sel }
      let nand0   = Nand { a: not_sel.out,  b: a0 }
      let nand1   = Nand { a: sel,          b: a1 }
      outputs.out = Nand { a: nand0.out, b: nand1.out }
     */
    fn expand(&self) -> Option<Vec<Project01Component>> {
        let not_sel = Not  { a: self.sel.clone(),             out: Output::new() };
        let nand0   = Nand { a: not_sel.out.clone().into(),   b: self.a0.clone(),       out: Output::new() };
        let nand1   = Nand { a: self.sel.clone(),             b: self.a1.clone(),       out: Output::new() };
        let out     = Nand { a: nand0.out.clone().into(),     b: nand1.out.clone().into(), out: self.out.clone() };
        Some(vec![not_sel.into(), nand0.into(), nand1.into(), out.into()])
    }
}

/// Routes input to a when sel is 0, or b when sel is 1; the unused output is zero.
#[derive(Reflect)]
pub struct Dmux {
    pub input: Input,
    pub sel: Input,
    pub a: Output,
    pub b: Output,
}
impl Component for Dmux {
    type Target = Project01Component;

    /*
      let not_sel = Not { a: inputs.sel }
      let and_a   = And { a: inputs.input, b: not_sel.out }
      let and_b   = And { a: inputs.input, b: inputs.sel  }
      outputs.a = and_a.out
      outputs.b = and_b.out
     */
    fn expand(&self) -> Option<Vec<Project01Component>> {
        let not_sel = Not { a: self.sel.clone(),   out: Output::new() };
        let and_a   = And { a: self.input.clone(), b: not_sel.out.clone().into(),   out: self.a.clone() };
        let and_b   = And { a: self.input.clone(), b: self.sel.clone(),   out: self.b.clone() };
        Some(vec![not_sel.into(), and_a.into(), and_b.into()])
    }
}

/// Inverts each bit of a 16-bit input.
#[derive(Reflect)]
pub struct Not16 {
    pub a: Input16,
    pub out: Output16,
}
impl Component for Not16 {
    type Target = Project01Component;

    /*
      for i in 0..16:
        let not = Not { a: inputs.a[i] }
        outputs.out[i] = not.out
     */
    fn expand(&self) -> Option<Vec<Project01Component>> {
        Some((0..16).map(|i| {
            Not { a: self.a.bit(i), out: self.out.bit(i) }.into()
        }).collect())
    }
}

/// Bitwise `And` across two 16-bit inputs.
#[derive(Reflect)]
pub struct And16 {
    pub a: Input16,
    pub b: Input16,
    pub out: Output16,
}
impl Component for And16 {
    type Target = Project01Component;

    /*
      for i in 0..16:
        let and = And { a: inputs.a[i], b: inputs.b[i] }
        outputs.out[i] = and.out
     */
    fn expand(&self) -> Option<Vec<Project01Component>> {
        Some((0..16).map(|i| {
            And { a: self.a.bit(i), b: self.b.bit(i), out: self.out.bit(i) }.into()
        }).collect())
    }
}

// /// Bitwise `Or` across two 16-bit inputs.
// #[derive(Reflect)]
// pub struct Or16 {
//     pub a: Input16,
//     pub b: Input16,
//     pub out: Output16,
// }
// impl Component for Or16 {
//     type Target = Project01Component;

//     /*
//       for i in 0..16:
//         let or = Or { a: inputs.a[i], b: inputs.b[i] }
//         outputs.out[i] = or.out
//      */
//     fn expand(&self) -> Option<Vec<Project01Component>> {
//         Some((0..16).map(|i| {
//             Or { a: self.a.bit(i), b: self.b.bit(i), out: self.out.bit(i) }.into()
//         }).collect())
//     }
// }

/// Selects between two 16-bit inputs bit-by-bit, using a single sel bit.
#[derive(Reflect)]
pub struct Mux16 {
    pub a0: Input16,
    pub a1: Input16,
    pub sel: Input,
    pub out: Output16,
}
impl Component for Mux16 {
    type Target = Project01Component;

    /*
      let not_sel = Not { a: sel }
      for i in 0..16:
        let nand0      = Nand { a: not_sel.out, b: a0[i]    }
        let nand1      = Nand { a: sel,         b: a1[i]    }
        outputs.out[i] = Nand { a: nand0.out,   b: nand1.out }
     */
    fn expand(&self) -> Option<Vec<Project01Component>> {
        let not_sel = Not { a: self.sel.clone(), out: Output::new() };
        let not_sel_out: Input = not_sel.out.clone().into();

        let mut result = vec![not_sel.into()];
        result.extend((0..16).flat_map(|i| {
            let nand0 = Nand { a: not_sel_out.clone(),       b: self.a0.bit(i),        out: Output::new() };
            let nand1 = Nand { a: self.sel.clone(),           b: self.a1.bit(i),        out: Output::new() };
            let out   = Nand { a: nand0.out.clone().into(),  b: nand1.out.clone().into(), out: self.out.bit(i) };
            vec![nand0.into(), nand1.into(), out.into()]
        }).collect::<Vec<_>>());
        Some(result)
    }
}


// These are needed for RAMs, maybe? Nevermind that stuff.
//   mux4way16
//   mux8way16
//   dmux4way
//   dmux8way
