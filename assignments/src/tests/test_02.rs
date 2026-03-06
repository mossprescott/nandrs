use simulator::{Input, Input16, Output, Output16};
use simulator::eval::eval;
use crate::project_02::{HalfAdder, FullAdder, Inc16, Add16};

#[test]
fn half_adder_truth_table() {
    let chip = HalfAdder { a: Input::new(), b: Input::new(), sum: Output::new(), carry: Output::new() };
    let r = eval(&chip, [("a", 0), ("b", 0)]); assert_eq!(r["sum"], 0); assert_eq!(r["carry"], 0);
    let r = eval(&chip, [("a", 0), ("b", 1)]); assert_eq!(r["sum"], 1); assert_eq!(r["carry"], 0);
    let r = eval(&chip, [("a", 1), ("b", 0)]); assert_eq!(r["sum"], 1); assert_eq!(r["carry"], 0);
    let r = eval(&chip, [("a", 1), ("b", 1)]); assert_eq!(r["sum"], 0); assert_eq!(r["carry"], 1);
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
fn inc16_truth_table() {
    let chip = Inc16 { in0: Input16::new(), out: Output16::new() };
    assert_eq!(eval(&chip, [("in0", 0)])["out"],     1);
    assert_eq!(eval(&chip, [("in0", 1)])["out"],     2);
    assert_eq!(eval(&chip, [("in0", 42)])["out"],    43);
    assert_eq!(eval(&chip, [("in0", 0xffff)])["out"], 0); // overflow wraps
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
