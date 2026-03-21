use crate::{Component, Input, Input1, Output, OutputBus, Reflect, expand, fixed};
use crate::declare::{Chip, Interface, BusRef};
use crate::component::{Buffer, Nand, Register, Sequential, Combinational};
use crate::nat::{N1, N8};


/// Really just about trivial component for testing the expand! macro.
pub struct TestNot {
    pub a: Input1,
    pub out: Output,
}

// Note: same as derived
impl Reflect for TestNot {
    fn reflect(&self) -> Interface {
        Interface {
            inputs:  [("a".to_string(),   BusRef::from_input(self.a))].into(),
            outputs: [("out".to_string(), BusRef::from_output(self.out))].into(),
        }
    }
    fn name(&self) -> String { "TestNot".to_string() }
}

// Note: same as derived
impl Chip for TestNot {
    fn chip() -> Self {
        TestNot { a: Input::new(), out: Output::new() }
    }
}

impl Component for TestNot {
    type Target = Combinational<N1>;

    expand! { |this| {
        nand: Nand {
            a: this.a,
            b: this.a,  // also a
            out: this.out
        },
    }}
}

#[test]
fn test_expand_not() {
    let chip = TestNot::chip();
    let ic = chip.expand().unwrap();

    assert_eq!(ic.name(), "TestNot");

    assert_eq!(ic.intf.inputs.len(), 1);
    let a = ic.intf.inputs["a"];

    assert_eq!(ic.intf.outputs.len(), 1);
    let out = ic.intf.outputs["out"];

    assert_eq!(ic.components.len(), 1);
    let Combinational::Nand(ref nand) = ic.components[0] else { panic!("expected Nand") };

    let nand_a   = BusRef::from_input(nand.a);
    let nand_b   = BusRef::from_input(nand.b);
    let nand_out = BusRef::from_output(nand.out);

    assert_eq!(nand_a.id,   a.id);
    assert_eq!(nand_b.id,   a.id);  // b is tied to a (it's a NOT: a NAND a)
    assert_eq!(nand_out.id, out.id);
}

/// Almost as trivial, but uses a second Nand.
pub struct TestAnd {
    pub a: Input1,
    pub b: Input1,
    pub out: Output,
}

impl Reflect for TestAnd {
    fn reflect(&self) -> Interface {
        Interface {
            inputs:  [("a".to_string(), BusRef::from_input(self.a)),
                      ("b".to_string(), BusRef::from_input(self.b))].into(),
            outputs: [("out".to_string(), BusRef::from_output(self.out))].into(),
        }
    }
    fn name(&self) -> String { "TestAnd".to_string() }
}

impl Chip for TestAnd {
    fn chip() -> Self {
        TestAnd { a: Input1::new(), b: Input1::new(), out: Output::new() }
    }
}

impl Component for TestAnd {
    type Target = Combinational<N1>;

    expand! { |this| {
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
    }}
}

#[test]
fn test_expand_and() {
    let chip = TestAnd::chip();
    let ic = chip.expand().unwrap();

    assert_eq!(ic.name(), "TestAnd");

    assert_eq!(ic.intf.inputs.len(), 2);
    let a = ic.intf.inputs["a"];
    let b = ic.intf.inputs["b"];

    assert_eq!(ic.intf.outputs.len(), 1);
    let out = ic.intf.outputs["out"];

    assert_eq!(ic.components.len(), 2);
    let Combinational::Nand(ref nand) = ic.components[0] else { panic!("expected Nand") };
    let nand_a   = BusRef::from_input(nand.a);
    let nand_b   = BusRef::from_input(nand.b);
    let nand_out = BusRef::from_output(nand.out);

    let Combinational::Nand(ref not) = ic.components[1] else { panic!("expected Nand") };
    let not_a   = BusRef::from_input(not.a);
    let not_b   = BusRef::from_input(not.b);
    let not_out = BusRef::from_output(not.out);

    assert_eq!(nand_a.id,   a.id);
    assert_eq!(nand_b.id,   b.id);

    assert_eq!(not_a.id, nand_out.id);
    assert_eq!(not_b.id, nand_out.id);

    assert_eq!(not_out.id, out.id);
}

/// A simple, bit-parallel component, for an uncommon data size.
pub struct TestNand8 {
    pub a: Input<N8>,
    pub b: Input<N8>,

    pub out: OutputBus<N8>,
}

impl Reflect for TestNand8 {
    fn reflect(&self) -> Interface {
        Interface {
            inputs:  [("a".to_string(), BusRef::from_input(self.a)),
                      ("b".to_string(), BusRef::from_input(self.b))].into(),
            outputs: [("out".to_string(), BusRef::from_output(self.out))].into(),
        }
    }
    fn name(&self) -> String { "TestNand8".to_string() }
}

impl Chip for TestNand8 {
    fn chip() -> Self {
        TestNand8 { a: Input::<N8>::new(), b: Input::<N8>::new(), out: OutputBus::<N8>::new() }
    }
}

impl Component for TestNand8 {
    type Target = Combinational<N8>;

    expand! { |this| {
        for i in 0..8 {
            _nand: Nand { a: this.a.bit(i).into(), b: this.b.bit(i).into(), out: this.out.bit(i) },
        }
    }}
}

#[test]
fn test_expand_nand8() {
    let chip = TestNand8::chip();
    let ic = chip.expand().unwrap();

    assert_eq!(ic.name(), "TestNand8");

    assert_eq!(ic.intf.inputs.len(), 2);

    assert_eq!(ic.intf.outputs.len(), 1);

    assert_eq!(ic.components.len(), 8);
}

/// A circuit that needs to refer to an output before its component is declared.
pub struct TestFlipFlop {
    pub out: Output,
}

impl Reflect for TestFlipFlop {
    fn reflect(&self) -> Interface {
        Interface {
            inputs:  [].into(),
            outputs: [("out".to_string(), BusRef::from_output(self.out))].into(),
        }
    }
    fn name(&self) -> String { "TestFlipFlop".to_string() }
}

impl Chip for TestFlipFlop {
    fn chip() -> Self {
        TestFlipFlop { out: Output::new() }
    }
}

impl Component for TestFlipFlop {
    type Target = Sequential<N1>;

    expand! { |this| {
        // Declare the register's output so we can refer to it circularly
        reg_out: forward Output::new(),

        not: Nand { a: reg_out.into(), b: this.out.into(), out: Output::new() },
        reg: Register { data_in: not.out.into(), write: fixed(1), data_out: reg_out },

        // Now connect to the chip output also
        _out: Buffer { a: reg_out.into(), out: this.out },
    }}
}

#[test]
fn test_expand_flip_flop() {
    let chip = TestFlipFlop::chip();
    let ic = chip.expand().unwrap();

    assert_eq!(ic.name(), "TestFlipFlop");

    assert_eq!(ic.intf.inputs.len(), 0);

    assert_eq!(ic.intf.outputs.len(), 1);
    let _out = ic.intf.outputs["out"];

    assert_eq!(ic.components.len(), 4);
    // TODO: verify wiring...
}

