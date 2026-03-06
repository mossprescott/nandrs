use simulator::{Component, Input, Input16, Output, Reflect};
use simulator::eval::eval;
use crate::project_01::{Nand, Not, And, Or, Xor, Mux, Dmux, Not16, And16, Or16, Mux16};

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
    assert_eq!(chip.expand().unwrap().len(), 1);
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
    assert_eq!(chip.expand().unwrap().len(), 2);
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
    assert_eq!(chip.expand().unwrap().len(), 3);
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
    assert_eq!(chip.expand().unwrap().len(), 3);
}

#[test]
fn mux_truth_table() {
    let chip = Mux { a: Input::new(), b: Input::new(), sel: Input::new(), out: Output::new() };
    assert_eq!(eval(&chip, [("a", 0), ("b", 0), ("sel", 0)])["out"], 0);
    assert_eq!(eval(&chip, [("a", 0), ("b", 1), ("sel", 0)])["out"], 0);
    assert_eq!(eval(&chip, [("a", 1), ("b", 0), ("sel", 0)])["out"], 1);
    assert_eq!(eval(&chip, [("a", 1), ("b", 1), ("sel", 0)])["out"], 1);
    assert_eq!(eval(&chip, [("a", 0), ("b", 0), ("sel", 1)])["out"], 0);
    assert_eq!(eval(&chip, [("a", 0), ("b", 1), ("sel", 1)])["out"], 1);
    assert_eq!(eval(&chip, [("a", 1), ("b", 0), ("sel", 1)])["out"], 0);
    assert_eq!(eval(&chip, [("a", 1), ("b", 1), ("sel", 1)])["out"], 1);
}

#[test]
fn mux_optimal() {
    let chip = Mux { a: Input::new(), b: Input::new(), sel: Input::new(), out: Output::new() };
    assert_eq!(chip.expand().unwrap().len(), 4);
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
    assert_eq!(chip.expand().unwrap().len(), 3);
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
    assert_eq!(chip.expand().unwrap().len(), 16);
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
    assert_eq!(chip.expand().unwrap().len(), 16);
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
    assert_eq!(chip.expand().unwrap().len(), 16);
}

#[test]
fn mux16_truth_table() {
    let chip = Mux16 { a: Input16::new(), b: Input16::new(), sel: Input::new(), out: Output::new() };
    assert_eq!(eval(&chip, [("a", 0xAAAA), ("b", 0x5555), ("sel", 0)])["out"], 0xAAAA);
    assert_eq!(eval(&chip, [("a", 0xAAAA), ("b", 0x5555), ("sel", 1)])["out"], 0x5555);
    assert_eq!(eval(&chip, [("a", 0x1234), ("b", 0x5678), ("sel", 0)])["out"], 0x1234);
    assert_eq!(eval(&chip, [("a", 0x1234), ("b", 0x5678), ("sel", 1)])["out"], 0x5678);
}

#[test]
fn mux16_optimal() {
    let chip = Mux16 { a: Input16::new(), b: Input16::new(), sel: Input::new(), out: Output::new() };
    assert_eq!(chip.expand().unwrap().len(), 16);
}
