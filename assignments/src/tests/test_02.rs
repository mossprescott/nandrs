use simulator::{Input, Input16, Output, Output16};
use simulator::eval::eval;
use crate::project_02::{flatten, HalfAdder, FullAdder, Inc16, Add16, Zero16, Neg16};

#[test]
fn half_adder_truth_table() {
    let chip = HalfAdder { a: Input::new(), b: Input::new(), sum: Output::new(), carry: Output::new() };
    let r = eval(&chip, [("a", 0), ("b", 0)]); assert_eq!(r["sum"], 0); assert_eq!(r["carry"], 0);
    let r = eval(&chip, [("a", 0), ("b", 1)]); assert_eq!(r["sum"], 1); assert_eq!(r["carry"], 0);
    let r = eval(&chip, [("a", 1), ("b", 0)]); assert_eq!(r["sum"], 1); assert_eq!(r["carry"], 0);
    let r = eval(&chip, [("a", 1), ("b", 1)]); assert_eq!(r["sum"], 0); assert_eq!(r["carry"], 1);
}

#[test]
fn half_adder_optimal() {
    let chip = HalfAdder { a: Input::new(), b: Input::new(), sum: Output::new(), carry: Output::new() };
    // TODO: 5, per nandgame
    assert_eq!(flatten(chip).len(), 6);
}

#[test]
fn full_adder_truth_table() {
    let chip = FullAdder { a: Input::new(), b: Input::new(), c: Input::new(), sum: Output::new(), carry: Output::new() };
    let r = eval(&chip, [("a", 0), ("b", 0), ("c", 0)]); assert_eq!(r["sum"], 0); assert_eq!(r["carry"], 0);
    let r = eval(&chip, [("a", 0), ("b", 0), ("c", 1)]); assert_eq!(r["sum"], 1); assert_eq!(r["carry"], 0);
    let r = eval(&chip, [("a", 0), ("b", 1), ("c", 0)]); assert_eq!(r["sum"], 1); assert_eq!(r["carry"], 0);
    let r = eval(&chip, [("a", 0), ("b", 1), ("c", 1)]); assert_eq!(r["sum"], 0); assert_eq!(r["carry"], 1);
    let r = eval(&chip, [("a", 1), ("b", 0), ("c", 0)]); assert_eq!(r["sum"], 1); assert_eq!(r["carry"], 0);
    let r = eval(&chip, [("a", 1), ("b", 0), ("c", 1)]); assert_eq!(r["sum"], 0); assert_eq!(r["carry"], 1);
    let r = eval(&chip, [("a", 1), ("b", 1), ("c", 0)]); assert_eq!(r["sum"], 0); assert_eq!(r["carry"], 1);
    let r = eval(&chip, [("a", 1), ("b", 1), ("c", 1)]); assert_eq!(r["sum"], 1); assert_eq!(r["carry"], 1);
}

#[test]
fn full_adder_optimal() {
    // 2 x HalfAdder(6) + Or(3) = 15
    let chip = FullAdder { a: Input::new(), b: Input::new(), c: Input::new(), sum: Output::new(), carry: Output::new() };
    // TODO: 9, per nandgame
    assert_eq!(flatten(chip).len(), 15);
}

#[test]
fn inc16_truth_table() {
    let chip = Inc16 { in0: Input16::new(), out: Output16::new() };
    assert_eq!(eval(&chip, [("in0", 0)])["out"],     1);
    assert_eq!(eval(&chip, [("in0", 1)])["out"],     2);
    assert_eq!(eval(&chip, [("in0", 42)])["out"],    43);
    assert_eq!(eval(&chip, [("in0", 0xffff)])["out"], 0); // overflow wraps
}

#[test]
fn inc16_optimal() {
    // Not(1) for bit 0 + 15 x HalfAdder(6) = 91
    let chip = Inc16 { in0: Input16::new(), out: Output16::new() };
    // TODO: pynand has 76
    assert_eq!(flatten(chip).len(), 91);
}

#[test]
fn add16_truth_table() {
    let chip = Add16 { a: Input16::new(), b: Input16::new(), out: Output16::new() };
    assert_eq!(eval(&chip, [("a", 0),    ("b", 0)])["out"],    0);
    assert_eq!(eval(&chip, [("a", 1),    ("b", 1)])["out"],    2);
    assert_eq!(eval(&chip, [("a", 100),  ("b", 200)])["out"],  300);
    assert_eq!(eval(&chip, [("a", 0xffff), ("b", 1)])["out"],  0); // overflow wraps

    // TODO: some examples for negative values by casting to/from i16
}

#[test]
fn add16_optimal() {
    // HalfAdder(6) for bit 0 + 15 x FullAdder(15) = 231
    let chip = Add16 { a: Input16::new(), b: Input16::new(), out: Output16::new() };
    // TODO: pynand has 140
    assert_eq!(flatten(chip).len(), 231);
}

#[test]
fn zero16_truth_table() {
    let chip = Zero16 { a: Input16::new(), out: Output::new() };
    assert_eq!(eval(&chip, [("a", 0)])["out"],      1); // all zeros
    assert_eq!(eval(&chip, [("a", 1)])["out"],      0); // bit 0 set
    assert_eq!(eval(&chip, [("a", 0x8000)])["out"], 0); // only MSB set
    assert_eq!(eval(&chip, [("a", 0xffff)])["out"], 0); // all ones
}

#[test]
fn zero16_optimal() {
    // Or-tree over 16 bits (15 Ors x 3 Nands) + Not(1) = 46
    let chip = Zero16 { a: Input16::new(), out: Output::new() };
    assert_eq!(flatten(chip).len(), 46);
}

#[test]
fn neg16_truth_table() {
    let chip = Neg16 { a: Input16::new(), out: Output::new() };
    assert_eq!(eval(&chip, [("a", 0)])["out"],      0); // zero is not negative
    assert_eq!(eval(&chip, [("a", 1)])["out"],      0); // positive
    assert_eq!(eval(&chip, [("a", 0x7fff)])["out"], 0); // max positive
    assert_eq!(eval(&chip, [("a", 0x8000)])["out"], 1); // min negative (-32768)
    assert_eq!(eval(&chip, [("a", 0xffff)])["out"], 1); // -1
}

// TODO: currently wasting a couple of gates
// #[test]
// fn neg16_optimal() {
//     // Not(Not(a[15])) = 2 Nands
//     let chip = Neg16 { a: Input16::new(), out: Output::new() };
//     assert_eq!(flatten(chip).len(), 0);
// }
