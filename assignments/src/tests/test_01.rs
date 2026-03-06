use simulator::{Input, Input16, Output, Output16, Reflect, print_graph};
use simulator::eval::eval;
use crate::project_01::{flatten, Nand, Not, And, Or, Xor, Mux, Dmux, Not16, And16, Or16, Mux16};

#[test]
fn nand_reflect() {
    let chip = Nand { a: Input::new(), b: Input::new(), out: Output::new() };
    let intf = chip.reflect();
    assert_eq!(intf.inputs.len(), 2);
    assert_eq!(intf.inputs["a"].width, 1);
    assert_eq!(intf.inputs["b"].width, 1);
    assert_eq!(intf.outputs.len(), 1);
    assert_eq!(intf.outputs["out"].width, 1);
}

#[test]
fn nand_truth_table() {
    let chip = Nand { a: Input::new(), b: Input::new(), out: Output::new() };
    assert_eq!(eval(&chip, [("a", 0), ("b", 0)])["out"], 1);
    assert_eq!(eval(&chip, [("a", 0), ("b", 1)])["out"], 1);
    assert_eq!(eval(&chip, [("a", 1), ("b", 0)])["out"], 1);
    assert_eq!(eval(&chip, [("a", 1), ("b", 1)])["out"], 0);
}


#[test]
fn not_truth_table() {
    let chip = Not { a: Input::new(), out: Output::new() };
    assert_eq!(eval(&chip, [("a", 0)])["out"], 1);
    assert_eq!(eval(&chip, [("a", 1)])["out"], 0);
}

#[test]
fn not_optimal() {
    let chip = Not { a: Input::new(), out: Output::new() };
    assert_eq!(flatten(chip).len(), 1);
}

#[test]
fn and_truth_table() {
    let chip = And { a: Input::new(), b: Input::new(), out: Output::new() };
    assert_eq!(eval(&chip, [("a", 0), ("b", 0)])["out"], 0);
    assert_eq!(eval(&chip, [("a", 0), ("b", 1)])["out"], 0);
    assert_eq!(eval(&chip, [("a", 1), ("b", 0)])["out"], 0);
    assert_eq!(eval(&chip, [("a", 1), ("b", 1)])["out"], 1);
}

#[test]
fn and_optimal() {
    let chip = And { a: Input::new(), b: Input::new(), out: Output::new() };
    assert_eq!(flatten(chip).len(), 2);
}

#[test]
fn or_truth_table() {
    let chip = Or { a: Input::new(), b: Input::new(), out: Output::new() };
    assert_eq!(eval(&chip, [("a", 0), ("b", 0)])["out"], 0);
    assert_eq!(eval(&chip, [("a", 0), ("b", 1)])["out"], 1);
    assert_eq!(eval(&chip, [("a", 1), ("b", 0)])["out"], 1);
    assert_eq!(eval(&chip, [("a", 1), ("b", 1)])["out"], 1);
}

#[test]
fn or_optimal() {
    let chip = Or { a: Input::new(), b: Input::new(), out: Output::new() };
    assert_eq!(flatten(chip).len(), 3);
}

#[test]
fn xor_truth_table() {
    let chip = Xor { a: Input::new(), b: Input::new(), out: Output::new() };
    assert_eq!(eval(&chip, [("a", 0), ("b", 0)])["out"], 0);
    assert_eq!(eval(&chip, [("a", 0), ("b", 1)])["out"], 1);
    assert_eq!(eval(&chip, [("a", 1), ("b", 0)])["out"], 1);
    assert_eq!(eval(&chip, [("a", 1), ("b", 1)])["out"], 0);
}

#[test]
fn xor_optimal() {
    let chip = Xor { a: Input::new(), b: Input::new(), out: Output::new() };
    assert_eq!(flatten(chip).len(), 6);
}

#[test]
fn mux_truth_table() {
    let chip = Mux { a0: Input::new(), a1: Input::new(), sel: Input::new(), out: Output::new() };
    assert_eq!(eval(&chip, [("a0", 0), ("a1", 0), ("sel", 0)])["out"], 0);
    assert_eq!(eval(&chip, [("a0", 0), ("a1", 1), ("sel", 0)])["out"], 0);
    assert_eq!(eval(&chip, [("a0", 1), ("a1", 0), ("sel", 0)])["out"], 1);
    assert_eq!(eval(&chip, [("a0", 1), ("a1", 1), ("sel", 0)])["out"], 1);
    assert_eq!(eval(&chip, [("a0", 0), ("a1", 0), ("sel", 1)])["out"], 0);
    assert_eq!(eval(&chip, [("a0", 0), ("a1", 1), ("sel", 1)])["out"], 1);
    assert_eq!(eval(&chip, [("a0", 1), ("a1", 0), ("sel", 1)])["out"], 0);
    assert_eq!(eval(&chip, [("a0", 1), ("a1", 1), ("sel", 1)])["out"], 1);
}

#[test]
fn mux_optimal() {
    let chip = Mux { a0: Input::new(), a1: Input::new(), sel: Input::new(), out: Output::new() };
    assert_eq!(flatten(chip).len(), 4);
}

#[test]
fn dmux_truth_table() {
    let chip = Dmux { input: Input::new(), sel: Input::new(), a: Output::new(), b: Output::new() };
    let r = eval(&chip, [("input", 0), ("sel", 0)]);
    assert_eq!((r["a"], r["b"]), (0, 0));
    let r = eval(&chip, [("input", 1), ("sel", 0)]);
    assert_eq!((r["a"], r["b"]), (1, 0));
    let r = eval(&chip, [("input", 0), ("sel", 1)]);
    assert_eq!((r["a"], r["b"]), (0, 0));
    let r = eval(&chip, [("input", 1), ("sel", 1)]);
    assert_eq!((r["a"], r["b"]), (0, 1));
}

#[test]
fn dmux_optimal() {
    let chip = Dmux { input: Input::new(), sel: Input::new(), a: Output::new(), b: Output::new() };
    assert_eq!(flatten(chip).len(), 5);
}

#[test]
fn not16_truth_table() {
    let chip = Not16 { a: Input16::new(), out: Output::new() };
    assert_eq!(eval(&chip, [("a", 0x0000)])["out"], 0xFFFF);
    assert_eq!(eval(&chip, [("a", 0xFFFF)])["out"], 0x0000);
    assert_eq!(eval(&chip, [("a", 0xAAAA)])["out"], 0x5555);
    assert_eq!(eval(&chip, [("a", 0x1234)])["out"], 0xEDCB);
}

#[test]
fn not16_optimal() {
    let chip = Not16 { a: Input16::new(), out: Output::new() };
    assert_eq!(flatten(chip).len(), 16);
}

#[test]
fn and16_truth_table() {
    let chip = And16 { a: Input16::new(), b: Input16::new(), out: Output::new() };
    assert_eq!(eval(&chip, [("a", 0xFFFF), ("b", 0xAAAA)])["out"], 0xAAAA);
    assert_eq!(eval(&chip, [("a", 0x0000), ("b", 0xFFFF)])["out"], 0x0000);
    assert_eq!(eval(&chip, [("a", 0xFF00), ("b", 0x0FF0)])["out"], 0x0F00);
    assert_eq!(eval(&chip, [("a", 0xFFFF), ("b", 0xFFFF)])["out"], 0xFFFF);
}

#[test]
fn and16_optimal() {
    let chip = And16 { a: Input16::new(), b: Input16::new(), out: Output::new() };
    assert_eq!(flatten(chip).len(), 32);
}

#[test]
fn or16_truth_table() {
    let chip = Or16 { a: Input16::new(), b: Input16::new(), out: Output::new() };
    assert_eq!(eval(&chip, [("a", 0x0000), ("b", 0xAAAA)])["out"], 0xAAAA);
    assert_eq!(eval(&chip, [("a", 0x5555), ("b", 0xAAAA)])["out"], 0xFFFF);
    assert_eq!(eval(&chip, [("a", 0xFF00), ("b", 0x00FF)])["out"], 0xFFFF);
    assert_eq!(eval(&chip, [("a", 0x1234), ("b", 0x0F0F)])["out"], 0x1F3F);
}

#[test]
fn or16_optimal() {
    let chip = Or16 { a: Input16::new(), b: Input16::new(), out: Output::new() };
    assert_eq!(flatten(chip).len(), 48);
}

#[test]
fn mux16_truth_table() {
    let chip = Mux16 { a0: Input16::new(), a1: Input16::new(), sel: Input::new(), out: Output::new() };
    assert_eq!(eval(&chip, [("a0", 0xAAAA), ("a1", 0x5555), ("sel", 0)])["out"], 0xAAAA);
    assert_eq!(eval(&chip, [("a0", 0xAAAA), ("a1", 0x5555), ("sel", 1)])["out"], 0x5555);
    assert_eq!(eval(&chip, [("a0", 0x1234), ("a1", 0x5678), ("sel", 0)])["out"], 0x1234);
    assert_eq!(eval(&chip, [("a0", 0x1234), ("a1", 0x5678), ("sel", 1)])["out"], 0x5678);
}

#[test]
fn mux16_optimal() {
    let chip = Mux16 { a0: Input16::new(), a1: Input16::new(), sel: Input::new(), out: Output::new() };
    // TODO: definitely can be improved, but a simple simplifier might take care of redundant gates
    // anyway.
    assert_eq!(flatten(chip).len(), 128);
}

#[test]
fn and_graph() {
    let chip = And { a: Input::new(), b: Input::new(), out: Output::new() };
    assert_eq!(
        print_graph(&chip),
        "And:\n\
         a -> nand0.a\n\
         b -> nand0.b\n\
         nand0.out -> not1.a\n\
         not1.out -> out"
    );
}

#[test]
fn mux16_graph() {
    let chip = Mux16 { a0: Input16::new(), a1: Input16::new(), sel: Input::new(), out: Output16::new() };
    assert_eq!(
        print_graph(&chip),
        "Mux16:\n\
         a0[0] -> mux0.a0\n\
         a0[1] -> mux1.a0\n\
         a0[2] -> mux2.a0\n\
         a0[3] -> mux3.a0\n\
         a0[4] -> mux4.a0\n\
         a0[5] -> mux5.a0\n\
         a0[6] -> mux6.a0\n\
         a0[7] -> mux7.a0\n\
         a0[8] -> mux8.a0\n\
         a0[9] -> mux9.a0\n\
         a0[10] -> mux10.a0\n\
         a0[11] -> mux11.a0\n\
         a0[12] -> mux12.a0\n\
         a0[13] -> mux13.a0\n\
         a0[14] -> mux14.a0\n\
         a0[15] -> mux15.a0\n\
         a1[0] -> mux0.a1\n\
         a1[1] -> mux1.a1\n\
         a1[2] -> mux2.a1\n\
         a1[3] -> mux3.a1\n\
         a1[4] -> mux4.a1\n\
         a1[5] -> mux5.a1\n\
         a1[6] -> mux6.a1\n\
         a1[7] -> mux7.a1\n\
         a1[8] -> mux8.a1\n\
         a1[9] -> mux9.a1\n\
         a1[10] -> mux10.a1\n\
         a1[11] -> mux11.a1\n\
         a1[12] -> mux12.a1\n\
         a1[13] -> mux13.a1\n\
         a1[14] -> mux14.a1\n\
         a1[15] -> mux15.a1\n\
         sel -> mux0.sel\n\
         sel -> mux1.sel\n\
         sel -> mux2.sel\n\
         sel -> mux3.sel\n\
         sel -> mux4.sel\n\
         sel -> mux5.sel\n\
         sel -> mux6.sel\n\
         sel -> mux7.sel\n\
         sel -> mux8.sel\n\
         sel -> mux9.sel\n\
         sel -> mux10.sel\n\
         sel -> mux11.sel\n\
         sel -> mux12.sel\n\
         sel -> mux13.sel\n\
         sel -> mux14.sel\n\
         sel -> mux15.sel\n\
         mux0.out -> out[0]\n\
         mux1.out -> out[1]\n\
         mux2.out -> out[2]\n\
         mux3.out -> out[3]\n\
         mux4.out -> out[4]\n\
         mux5.out -> out[5]\n\
         mux6.out -> out[6]\n\
         mux7.out -> out[7]\n\
         mux8.out -> out[8]\n\
         mux9.out -> out[9]\n\
         mux10.out -> out[10]\n\
         mux11.out -> out[11]\n\
         mux12.out -> out[12]\n\
         mux13.out -> out[13]\n\
         mux14.out -> out[14]\n\
         mux15.out -> out[15]"
    );
}
