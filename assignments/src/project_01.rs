#![allow(unused_variables, dead_code, unused_imports)]

use simulator::{self, Assembly, Component, Input, Output};
use std::collections::HashMap;

/// The single primitive
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

    fn reflect(&self) -> simulator::Interface {
        simulator::Interface {
            inputs: HashMap::from([
                ("a".to_string(),   self.a.clone().into()),
                ("b".to_string(),   self.b.clone().into()),
            ]),
            outputs: HashMap::from([
                ("out".to_string(), self.out.clone().into()),
            ]),
        }
    }
}

/// Components implemented in this project: simple, logical components for 1 and 16 bits.
pub enum Project01Component {
    Nand(Nand),
    Not(Not),
    And(And),
}

impl From<Nand> for Project01Component {
    fn from(c: Nand) -> Self { Project01Component::Nand(c) }
}

impl From<Not> for Project01Component {
    fn from(c: Not) -> Self { Project01Component::Not(c) }
}

impl From<And> for Project01Component {
    fn from(c: And) -> Self { Project01Component::And(c) }
}

impl Component for Project01Component {
    type Target = Project01Component;

    fn expand(&self) -> Option<Vec<Project01Component>> {
        match self {
            Project01Component::Nand(c) => c.expand().map(|v| v.into_iter().map(Into::into).collect()),
            Project01Component::Not(c)  => c.expand(),
            Project01Component::And(c)  => c.expand(),
        }
    }

    fn reflect(&self) -> simulator::Interface {
        match self {
            Project01Component::Nand(c) => c.reflect(),
            Project01Component::Not(c)  => c.reflect(),
            Project01Component::And(c)  => c.reflect(),
        }
    }
}


pub struct Not {
    pub a: Input,
    pub out: Output,
}
impl Component for Not {
    type Target = Project01Component;

    fn expand(&self) -> Option<Vec<Project01Component>> {
        let nand = Nand {
            a: self.a.clone(),
            b: self.a.clone(),
            out: self.out.clone(),
        };
        Option::Some(vec![nand.into()])
    }

    fn reflect(&self) -> simulator::Interface {
        simulator::Interface {
            inputs: HashMap::from([
                ("a".to_string(), self.a.clone().into()),
            ]),
            outputs: HashMap::from([
                ("out".to_string(), self.out.clone().into()),
            ]),
        }
    }
}

pub struct And {
    pub a: Input,
    pub b: Input,
    pub out: Output,
}
impl Component for And {
    type Target = Project01Component;

    fn expand(&self) -> Option<Vec<Project01Component>> {
        // Use an intermediate wire so nand.out isn't moved before nand is moved into the vec.
        let wire = Output::new();
        let nand = Nand {
            a: self.a.clone(),
            b: self.b.clone(),
            out: wire.clone(),
        };
        let not = Not {
            a: wire.into(),
            out: self.out.clone(),
        };
        Option::Some(vec![nand.into(), not.into()])
    }

    fn reflect(&self) -> simulator::Interface {
        simulator::Interface {
            inputs: HashMap::from([
                ("a".to_string(), self.a.clone().into()),
                ("b".to_string(), self.b.clone().into()),
            ]),
            outputs: HashMap::from([
                ("out".to_string(), self.out.clone().into()),
            ]),
        }
    }
}

pub fn or(a: bool, b: bool) -> bool {
    todo!()
}

pub fn xor(a: bool, b: bool) -> bool {
    todo!()
}

/// If sel is false, output a. If sel is true, output b.
pub fn mux(a: bool, b: bool, sel: bool) -> bool {
    todo!()
}

/// If sel is false, output (a, false). If sel is true, output (false, b).
pub fn dmux(input: bool, sel: bool) -> (bool, bool) {
    todo!()
}

pub fn not16(a: [bool; 16]) -> [bool; 16] {
    todo!()
}

pub fn and16(a: [bool; 16], b: [bool; 16]) -> [bool; 16] {
    todo!()
}

pub fn or16(a: [bool; 16], b: [bool; 16]) -> [bool; 16] {
    todo!()
}

pub fn mux16(a: [bool; 16], b: [bool; 16], sel: bool) -> [bool; 16] {
    todo!()
}

/// True if any bit in the input is true.
pub fn or8way(input: [bool; 8]) -> bool {
    todo!()
}

pub fn mux4way16(a: [bool; 16], b: [bool; 16], c: [bool; 16], d: [bool; 16], sel: [bool; 2]) -> [bool; 16] {
    todo!()
}

pub fn mux8way16(
    a: [bool; 16], b: [bool; 16], c: [bool; 16], d: [bool; 16],
    e: [bool; 16], f: [bool; 16], g: [bool; 16], h: [bool; 16],
    sel: [bool; 3],
) -> [bool; 16] {
    todo!()
}

pub fn dmux4way(input: bool, sel: [bool; 2]) -> (bool, bool, bool, bool) {
    todo!()
}

pub fn dmux8way(input: bool, sel: [bool; 3]) -> (bool, bool, bool, bool, bool, bool, bool, bool) {
    todo!()
}
