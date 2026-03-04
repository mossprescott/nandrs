use simulator::{Component, Input, Output};
use simulator::eval::eval;
use crate::project_01::{Nand, Not, And};

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

// #[test]
// fn or_truth_table() {
//     assert_eq!(or(false, false), false);
//     assert_eq!(or(false, true),  true);
//     assert_eq!(or(true,  false), true);
//     assert_eq!(or(true,  true),  true);
// }

// #[test]
// fn xor_truth_table() {
//     assert_eq!(xor(false, false), false);
//     assert_eq!(xor(false, true),  true);
//     assert_eq!(xor(true,  false), true);
//     assert_eq!(xor(true,  true),  false);
// }

// #[test]
// fn mux_truth_table() {
//     assert_eq!(mux(false, false, false), false);
//     assert_eq!(mux(false, true,  false), false);
//     assert_eq!(mux(true,  false, false), true);
//     assert_eq!(mux(true,  true,  false), true);
//     assert_eq!(mux(false, false, true),  false);
//     assert_eq!(mux(false, true,  true),  true);
//     assert_eq!(mux(true,  false, true),  false);
//     assert_eq!(mux(true,  true,  true),  true);
// }

// #[test]
// fn dmux_truth_table() {
//     assert_eq!(dmux(false, false), (false, false));
//     assert_eq!(dmux(true,  false), (true,  false));
//     assert_eq!(dmux(false, true),  (false, false));
//     assert_eq!(dmux(true,  true),  (false, true));
// }
