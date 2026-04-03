use crate::component::{Buffer, Nand, Register};
use crate::declare::{BusRef, Component, Interface};
use crate::nat::N1;
use crate::{
    Chip, IC, Input, Input1, Output, OutputBus, Reflect, expand, fixed, print_graph, print_ic_graph,
};
use frunk::Coprod;

/// Really just about trivial component for testing the expand! macro.
#[derive(Clone, Reflect, Chip)]
pub struct TestNot {
    pub a: Input1,
    pub out: Output,
}
impl Component for TestNot {
    type Target = Coprod!(Nand);

    fn define(&self) -> IC<Self::Target> {
        self.expand()
    }
}
impl TestNot {
    expand!([Nand], |this| {
        nand: Nand {
            a: this.a,
            b: this.a,  // also a
            out: this.out
        },
    });
}

#[test]
fn test_expand_not() {
    let chip = TestNot::chip();

    assert_eq!(chip.define().components.len(), 1);
    assert_eq!(
        print_graph(&chip),
        "TestNot:\n  nand_0.a <- a\n  nand_0.b <- a\n  out <- nand_0.out"
    );
}

/// Almost as trivial, but uses a second Nand.
#[derive(Clone, Reflect, Chip)]
pub struct TestAnd {
    pub a: Input1,
    pub b: Input1,
    pub out: Output,
}
impl Component for TestAnd {
    type Target = Coprod!(Nand);

    fn define(&self) -> IC<Self::Target> {
        self.expand()
    }
}
impl TestAnd {
    expand!([Nand], |this| {
        nand: Nand {
            a: this.a,
            b: this.b,
            out: Output::new(),
        },
        not: Nand {
            a: nand.out.into(),
            b: nand.out.into(),
            out: this.out,
        }
    });
}

#[test]
fn test_expand_and() {
    let chip = TestAnd::chip();

    assert_eq!(chip.define().components.len(), 2);
    assert_eq!(
        print_graph(&chip),
        "TestAnd:
  nand_0.a <- a
  nand_0.b <- b
  nand_1.a <- nand_0.out
  nand_1.b <- nand_0.out
  out <- nand_1.out"
    );
}

/// A simple, bit-parallel component, for an uncommon data size.
#[derive(Clone, Reflect, Chip)]
pub struct TestNand8 {
    pub a: Input<N8>,
    pub b: Input<N8>,

    pub out: OutputBus<N8>,
}
impl TestNand8 {
    expand!([Nand], |this| {
        for i in 0..8 {
            _nand: Nand { a: this.a.bit(i).into(), b: this.b.bit(i).into(), out: this.out.bit(i) },
        }
    });
}

use crate::nat::N8;
type TestNand8T = Coprod!(Nand);

#[test]
fn test_expand_nand8() {
    let chip = TestNand8::chip();
    let ic = chip.expand::<TestNand8T, _>();

    assert_eq!(ic.name(), "TestNand8");
    assert_eq!(ic.intf.inputs.len(), 2);
    assert_eq!(ic.intf.outputs.len(), 1);
    assert_eq!(ic.components.len(), 8);
}

type Register1 = Register<N1>;

/// A circuit that needs to refer to an output before its component is declared.
#[derive(Clone, Reflect, Chip)]
pub struct TestFlipFlop {
    pub out: Output,
}
impl TestFlipFlop {
    expand!([Nand, Buffer, Register1], |this| {
        // Declare the register's output so we can refer to it circularly
        reg_out: forward Output::new(),

        not: Nand { a: reg_out.into(), b: this.out.into(), out: Output::new() },
        reg: Register { data_in: not.out.into(), write: fixed(1), data_out: reg_out },

        // Now connect to the chip output also
        _out: Buffer { a: reg_out.into(), out: this.out },
    });
}

type TestFlipFlopT = Coprod!(Nand, Buffer, Register1);

#[test]
fn test_expand_flip_flop() {
    let chip = TestFlipFlop::chip();
    let ic = chip.expand::<TestFlipFlopT, _, _, _>();

    assert_eq!(ic.name(), "TestFlipFlop");
    assert_eq!(ic.intf.inputs.len(), 0);
    assert_eq!(ic.intf.outputs.len(), 1);
    assert_eq!(ic.components.len(), 3);
}
