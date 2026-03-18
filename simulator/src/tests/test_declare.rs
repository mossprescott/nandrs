use crate::{Component, Input, Output, Reflect, expand};
use crate::declare::{Chip, Interface, BusRef};
use crate::component::{Nand, Combinational};
use crate::nat::N1;


/// Really just about trivial component for testing the expand! macro.
pub struct TestNot {
    pub a: Input,
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
        nand: Nand { a: this.a.clone(), b: this.a.clone(), out: this.out.clone() }
    }}
}

#[test]
fn test_expand_macro() {
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