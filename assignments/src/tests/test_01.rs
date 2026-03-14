use simulator::{Input, Output, Reflect, print_graph};
use simulator::Chip as _;
use simulator::eval::eval;
use crate::project_01::{flatten, Const, Nand, Not, And, Or, Xor, Mux, Dmux, Not16, And16, Mux16};

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
    let chip = flatten(Nand::chip());
    assert_eq!(eval(&chip, [("a", 0), ("b", 0)])["out"], 1);
    assert_eq!(eval(&chip, [("a", 0), ("b", 1)])["out"], 1);
    assert_eq!(eval(&chip, [("a", 1), ("b", 0)])["out"], 1);
    assert_eq!(eval(&chip, [("a", 1), ("b", 1)])["out"], 0);
}

#[test]
fn const_truth_table() {
    let chip = flatten(Const::chip(0));
    assert_eq!(eval(&chip, [])["out"], 0);

    let chip = flatten(Const::chip(1234));
    assert_eq!(eval(&chip, [])["out"], 1234);
}

#[test]
fn not_truth_table() {
    let chip = flatten(Not::chip());
    assert_eq!(eval(&chip, [("a", 0)])["out"], 1);
    assert_eq!(eval(&chip, [("a", 1)])["out"], 0);
}

#[test]
fn not_optimal() {
    assert_eq!(flatten(Not::chip()).components.len(), 1);
}

#[test]
fn and_truth_table() {
    let chip = flatten(And::chip());
    assert_eq!(eval(&chip, [("a", 0), ("b", 0)])["out"], 0);
    assert_eq!(eval(&chip, [("a", 0), ("b", 1)])["out"], 0);
    assert_eq!(eval(&chip, [("a", 1), ("b", 0)])["out"], 0);
    assert_eq!(eval(&chip, [("a", 1), ("b", 1)])["out"], 1);
}

#[test]
fn and_optimal() {
    assert_eq!(flatten(And::chip()).components.len(), 2);
}

#[test]
fn or_truth_table() {
    let chip = flatten(Or::chip());
    assert_eq!(eval(&chip, [("a", 0), ("b", 0)])["out"], 0);
    assert_eq!(eval(&chip, [("a", 0), ("b", 1)])["out"], 1);
    assert_eq!(eval(&chip, [("a", 1), ("b", 0)])["out"], 1);
    assert_eq!(eval(&chip, [("a", 1), ("b", 1)])["out"], 1);
}

#[test]
fn or_optimal() {
    assert_eq!(flatten(Or::chip()).components.len(), 3);
}

#[test]
fn xor_truth_table() {
    let chip = flatten(Xor::chip());
    assert_eq!(eval(&chip, [("a", 0), ("b", 0)])["out"], 0);
    assert_eq!(eval(&chip, [("a", 0), ("b", 1)])["out"], 1);
    assert_eq!(eval(&chip, [("a", 1), ("b", 0)])["out"], 1);
    assert_eq!(eval(&chip, [("a", 1), ("b", 1)])["out"], 0);
}

#[test]
fn xor_optimal() {
    assert_eq!(flatten(Xor::chip()).components.len(), 4);
}

// Sanity check
#[test]
fn mux_truth_table() {
    let chip = flatten(Mux::chip());
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
    // Mux is now a primitive; flatten just returns it as-is.
    assert_eq!(flatten(Mux::chip()).components.len(), 1);
}

#[test]
fn dmux_truth_table() {
    let chip = flatten(Dmux::chip());
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
    assert_eq!(flatten(Dmux::chip()).components.len(), 5);
}

#[test]
fn not16_truth_table() {
    let chip = flatten(Not16::chip());
    assert_eq!(eval(&chip, [("a", 0x0000)])["out"], 0xFFFF);
    assert_eq!(eval(&chip, [("a", 0xFFFF)])["out"], 0x0000);
    assert_eq!(eval(&chip, [("a", 0xAAAA)])["out"], 0x5555);
    assert_eq!(eval(&chip, [("a", 0x1234)])["out"], 0xEDCB);
}

#[test]
fn not16_optimal() {
    assert_eq!(flatten(Not16::chip()).components.len(), 16);
}

#[test]
fn and16_truth_table() {
    let chip = flatten(And16::chip());
    assert_eq!(eval(&chip, [("a", 0xFFFF), ("b", 0xAAAA)])["out"], 0xAAAA);
    assert_eq!(eval(&chip, [("a", 0x0000), ("b", 0xFFFF)])["out"], 0x0000);
    assert_eq!(eval(&chip, [("a", 0xFF00), ("b", 0x0FF0)])["out"], 0x0F00);
    assert_eq!(eval(&chip, [("a", 0xFFFF), ("b", 0xFFFF)])["out"], 0xFFFF);
}

#[test]
fn and16_optimal() {
    assert_eq!(flatten(And16::chip()).components.len(), 32);
}

// #[test]
// fn or16_truth_table() {
//     let chip = flatten(Or16::chip());
//     assert_eq!(eval(&chip, [("a", 0x0000), ("b", 0xAAAA)])["out"], 0xAAAA);
//     assert_eq!(eval(&chip, [("a", 0x5555), ("b", 0xAAAA)])["out"], 0xFFFF);
//     assert_eq!(eval(&chip, [("a", 0xFF00), ("b", 0x00FF)])["out"], 0xFFFF);
//     assert_eq!(eval(&chip, [("a", 0x1234), ("b", 0x0F0F)])["out"], 0x1F3F);
// }

// #[test]
// fn or16_optimal() {
//     assert_eq!(flatten(Or16::chip()).components.len(), 48);
// }

/// Sanity check
#[test]
fn mux16_truth_table() {
    let chip = flatten(Mux16::chip());
    assert_eq!(eval(&chip, [("a0", 0xAAAA), ("a1", 0x5555), ("sel", 0)])["out"], 0xAAAA);
    assert_eq!(eval(&chip, [("a0", 0xAAAA), ("a1", 0x5555), ("sel", 1)])["out"], 0x5555);
    assert_eq!(eval(&chip, [("a0", 0x1234), ("a1", 0x5678), ("sel", 0)])["out"], 0x1234);
    assert_eq!(eval(&chip, [("a0", 0x1234), ("a1", 0x5678), ("sel", 1)])["out"], 0x5678);
}

#[test]
fn mux16_optimal() {
    // Mux16 is now a primitive; flatten just returns it as-is.
    assert_eq!(flatten(Mux16::chip()).components.len(), 1);
}

#[test]
fn and_graph() {
    let chip = And::chip();
    assert_eq!(
        print_graph(&chip),
        "And:\n\
         nand_0.a <- a\n\
         nand_0.b <- b\n\
         not_1.a <- nand_0.out\n\
         out <- not_1.out"
    );
}
