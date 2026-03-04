use simulator::{Component, Input, Output};
use simulator::eval::eval;
use crate::project_01::{Nand, Not, And, Or, Xor, Mux, Dmux};

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
    assert_eq!(eval(&chip, [("a", false), ("b", false)])["out"], true);
    assert_eq!(eval(&chip, [("a", false), ("b", true) ])["out"], true);
    assert_eq!(eval(&chip, [("a", true),  ("b", false)])["out"], true);
    assert_eq!(eval(&chip, [("a", true),  ("b", true) ])["out"], false);
}


#[test]
fn not_truth_table() {
    let chip = Not { a: Input::new(), out: Output::new() };
    assert_eq!(eval(&chip, [("a", false)])["out"], true);
    assert_eq!(eval(&chip, [("a", true) ])["out"], false);
}

#[test]
fn not_optimal() {
    let chip = Not { a: Input::new(), out: Output::new() };
    assert_eq!(chip.expand().unwrap().len(), 1);
}

#[test]
fn and_truth_table() {
    let chip = And { a: Input::new(), b: Input::new(), out: Output::new() };
    assert_eq!(eval(&chip, [("a", false), ("b", false)])["out"], false);
    assert_eq!(eval(&chip, [("a", false), ("b", true) ])["out"], false);
    assert_eq!(eval(&chip, [("a", true),  ("b", false)])["out"], false);
    assert_eq!(eval(&chip, [("a", true),  ("b", true) ])["out"], true);
}

#[test]
fn and_optimal() {
    let chip = And { a: Input::new(), b: Input::new(), out: Output::new() };
    assert_eq!(chip.expand().unwrap().len(), 2);
}

#[test]
fn or_truth_table() {
    let chip = Or { a: Input::new(), b: Input::new(), out: Output::new() };
    assert_eq!(eval(&chip, [("a", false), ("b", false)])["out"], false);
    assert_eq!(eval(&chip, [("a", false), ("b", true) ])["out"], true);
    assert_eq!(eval(&chip, [("a", true),  ("b", false)])["out"], true);
    assert_eq!(eval(&chip, [("a", true),  ("b", true) ])["out"], true);
}

#[test]
fn or_optimal() {
    let chip = Or { a: Input::new(), b: Input::new(), out: Output::new() };
    assert_eq!(chip.expand().unwrap().len(), 3);
}

#[test]
fn xor_truth_table() {
    let chip = Xor { a: Input::new(), b: Input::new(), out: Output::new() };
    assert_eq!(eval(&chip, [("a", false), ("b", false)])["out"], false);
    assert_eq!(eval(&chip, [("a", false), ("b", true) ])["out"], true);
    assert_eq!(eval(&chip, [("a", true),  ("b", false)])["out"], true);
    assert_eq!(eval(&chip, [("a", true),  ("b", true) ])["out"], false);
}

#[test]
fn xor_optimal() {
    let chip = Xor { a: Input::new(), b: Input::new(), out: Output::new() };
    assert_eq!(chip.expand().unwrap().len(), 3);
}

#[test]
fn mux_truth_table() {
    let chip = Mux { a: Input::new(), b: Input::new(), sel: Input::new(), out: Output::new() };
    assert_eq!(eval(&chip, [("a", false), ("b", false), ("sel", false)])["out"], false);
    assert_eq!(eval(&chip, [("a", false), ("b", true),  ("sel", false)])["out"], false);
    assert_eq!(eval(&chip, [("a", true),  ("b", false), ("sel", false)])["out"], true);
    assert_eq!(eval(&chip, [("a", true),  ("b", true),  ("sel", false)])["out"], true);
    assert_eq!(eval(&chip, [("a", false), ("b", false), ("sel", true) ])["out"], false);
    assert_eq!(eval(&chip, [("a", false), ("b", true),  ("sel", true) ])["out"], true);
    assert_eq!(eval(&chip, [("a", true),  ("b", false), ("sel", true) ])["out"], false);
    assert_eq!(eval(&chip, [("a", true),  ("b", true),  ("sel", true) ])["out"], true);
}

#[test]
fn mux_optimal() {
    let chip = Mux { a: Input::new(), b: Input::new(), sel: Input::new(), out: Output::new() };
    assert_eq!(chip.expand().unwrap().len(), 4);
}

#[test]
fn dmux_truth_table() {
    let chip = Dmux { input: Input::new(), sel: Input::new(), a: Output::new(), b: Output::new() };
    let r = eval(&chip, [("input", false), ("sel", false)]);
    assert_eq!((r["a"], r["b"]), (false, false));
    let r = eval(&chip, [("input", true),  ("sel", false)]);
    assert_eq!((r["a"], r["b"]), (true,  false));
    let r = eval(&chip, [("input", false), ("sel", true) ]);
    assert_eq!((r["a"], r["b"]), (false, false));
    let r = eval(&chip, [("input", true),  ("sel", true) ]);
    assert_eq!((r["a"], r["b"]), (false, true));
}

#[test]
fn dmux_optimal() {
    let chip = Dmux { input: Input::new(), sel: Input::new(), a: Output::new(), b: Output::new() };
    assert_eq!(chip.expand().unwrap().len(), 3);
}
