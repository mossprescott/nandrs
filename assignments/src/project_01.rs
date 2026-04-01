#![allow(unused_variables, dead_code, unused_imports)]

use frunk::Coprod;
use frunk::coproduct::CoprodInjector;
use simulator::Chip as _;
use simulator::Reflect as _;
use simulator::component::Combinational;
use simulator::component::CombinationalT;
use simulator::declare::Input;
use simulator::declare::{BusRef, Interface};
use simulator::nat::{N1, N16};
use simulator::{self, Chip, Component, IC, Input1, Input16, Output, Output16, Reflect, expand};
use std::collections::HashMap;

// Re-export since the other components here parallel Nand:
pub use simulator::component::{Buffer, Nand};

/// Components implemented in this project: simple, logical components for 1 and 16 bits.
#[derive(Clone, Reflect, Component)]
pub enum Project01Component {
    #[primitive]
    Nand(Nand),
    #[primitive]
    Buffer(Buffer),
    Not(Not),
    And(And),
    Or(Or),
    Xor(Xor),
    Mux(Mux),
    Dmux(Dmux),
    Not16(Not16),
    And16(And16),
    Mux16(Mux16),
}

type Project01ComponentT = Coprod!(Nand, Buffer, Not, And);  // TODO: remaining components

/// Recursively expand() until only primitives are left.
pub fn flatten<C: Reflect + Into<Project01Component>>(chip: C) -> IC<Combinational> {
    fn go(comp: Project01Component) -> Vec<Combinational> {
        match comp.expand() {
            None => match comp {
                Project01Component::Nand(c) => vec![c.into()],
                Project01Component::Buffer(c) => vec![c.into()],
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

pub fn flatten_t<C, Idx>(chip: C) -> IC<CombinationalT>
where
    C: Reflect,
    Project01ComponentT: CoprodInjector<C, Idx>,
{
    use frunk::{Coproduct, hlist};

    fn go(comp: Project01ComponentT) -> Vec<CombinationalT> {
        comp.fold(hlist![
            |nand: Nand|     vec![Coproduct::inject(nand)],
            |buffer: Buffer| vec![Coproduct::inject(buffer)],
            |not: Not|       not.expand_t::<CombinationalT, _>().components,
            |and: And|       and.expand_t::<Project01ComponentT, _, _>().components.into_iter().flat_map(go).collect(),
        ])
    }
    IC {
        name: format!("{} (flat)", chip.name()),
        intf: chip.reflect(),
        components: go(Project01ComponentT::inject(chip)),
    }
}

/// Inverts its input.
#[derive(Clone, Reflect, Chip)]
pub struct Not {
    pub a: Input1,
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
impl Not {
    pub fn expand_t<C, NandIdx>(&self) -> IC<C>
    where
        C: CoprodInjector<Nand, NandIdx>,
    {
        IC {
            name: self.name(),
            intf: self.reflect(),
            components: vec![C::inject(Nand {
                a: self.a,
                b: self.a, // also the "a" input
                out: self.out,
            })],
        }
    }
}

/// True only when both inputs are true.
#[derive(Clone, Reflect, Chip)]
pub struct And {
    pub a: Input1,
    pub b: Input1,
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
impl And {
    pub fn expand_t<C, NandIdx, NotIdx>(&self) -> IC<C>
    where
        C: CoprodInjector<Nand, NandIdx>,
        C: CoprodInjector<Not, NotIdx>,
    {
        let nand = Nand { a: self.a, b: self.b, out: Output::new() };
        let not = Not { a: nand.out.into(), out: self.out };
        IC {
            name: self.name(),
            intf: self.reflect(),
            components: vec![
                C::inject(nand),
                C::inject(not),
            ],
        }
    }
}

/// True when at least one input is true.
#[derive(Clone, Reflect, Chip)]
pub struct Or {
    pub a: Input1,
    pub b: Input1,
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
    pub a: Input1,
    pub b: Input1,
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

/// Passes a0 through when sel is 0, a1 when sel is 1.
#[derive(Clone, Reflect, Chip)]
pub struct Mux {
    pub a0: Input1,
    pub a1: Input1,
    pub sel: Input1,
    pub out: Output,
}
impl Component for Mux {
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
    pub input: Input1,
    pub sel: Input1,
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

/// Selects between two 16-bit inputs bit-by-bit, using a single sel bit.
#[derive(Clone, Reflect, Chip)]
pub struct Mux16 {
    pub a0: Input16,
    pub a1: Input16,
    pub sel: Input1,
    pub out: Output16,
}

impl Component for Mux16 {
    type Target = Project01Component;

    expand! { |this| {
        not_sel: Not { a: this.sel, out: Output::new() },
        for i in 0..16 {
            nand0: Nand { a: not_sel.out.clone().into(), b: this.a0.bit(i),           out: Output::new() },
            nand1: Nand { a: this.sel,                   b: this.a1.bit(i),           out: Output::new() },
            _out:  Nand { a: nand0.out.clone().into(),   b: nand1.out.clone().into(), out: this.out.bit(i) }
        }
    }}
}
