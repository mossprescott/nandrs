use simulator::{Chip as _};
use simulator::eval::eval;
use crate::project_02::{flatten, HalfAdder, FullAdder, Inc16, Add16, Zero16, Neg16, Alu};

#[test]
fn half_adder_truth_table() {
    let chip = HalfAdder::chip();
    let r = eval(&chip, [("a", 0), ("b", 0)]); assert_eq!(r["sum"], 0); assert_eq!(r["carry"], 0);
    let r = eval(&chip, [("a", 0), ("b", 1)]); assert_eq!(r["sum"], 1); assert_eq!(r["carry"], 0);
    let r = eval(&chip, [("a", 1), ("b", 0)]); assert_eq!(r["sum"], 1); assert_eq!(r["carry"], 0);
    let r = eval(&chip, [("a", 1), ("b", 1)]); assert_eq!(r["sum"], 0); assert_eq!(r["carry"], 1);
}

#[test]
fn half_adder_optimal() {
    assert_eq!(flatten(HalfAdder::chip()).len(), 5);
}

#[test]
fn full_adder_truth_table() {
    let chip = FullAdder::chip();
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
    assert_eq!(flatten(FullAdder::chip()).len(), 9);
}

#[test]
fn inc16_truth_table() {
    let chip = Inc16::chip();
    assert_eq!(eval(&chip, [("a", 0)])["out"],     1);
    assert_eq!(eval(&chip, [("a", 1)])["out"],     2);
    assert_eq!(eval(&chip, [("a", 42)])["out"],    43);
    assert_eq!(eval(&chip, [("a", 0xffff)])["out"], 0); // overflow wraps
}

#[test]
fn inc16_optimal() {
    // Not(1) for bit 0 + 15 x HalfAdder(6) = 91
    // Not(1) + 15 x HalfAdder(5) = 76
    assert_eq!(flatten(Inc16::chip()).len(), 76);
}

#[test]
fn add16_truth_table() {
    let chip = Add16::chip();
    assert_eq!(eval(&chip, [("a", 0),    ("b", 0)])["out"],    0);
    assert_eq!(eval(&chip, [("a", 1),    ("b", 1)])["out"],    2);
    assert_eq!(eval(&chip, [("a", 100),  ("b", 200)])["out"],  300);
    assert_eq!(eval(&chip, [("a", 0xffff), ("b", 1)])["out"],  0); // overflow wraps

    // TODO: some examples for negative values by casting to/from i16
}

#[test]
fn add16_optimal() {
    // HalfAdder(5) + 15 x FullAdder(9) = 140
    assert_eq!(flatten(Add16::chip()).len(), 140);
}

#[test]
fn zero16_truth_table() {
    let chip = Zero16::chip();
    assert_eq!(eval(&chip, [("a", 0)])["out"],      1); // all zeros
    assert_eq!(eval(&chip, [("a", 1)])["out"],      0); // bit 0 set
    assert_eq!(eval(&chip, [("a", 0x8000)])["out"], 0); // only MSB set
    assert_eq!(eval(&chip, [("a", 0xffff)])["out"], 0); // all ones
}

#[test]
fn zero16_optimal() {
    // Or-tree over 16 bits (15 Ors x 3 Nands) + Not(1) = 46
    assert_eq!(flatten(Zero16::chip()).len(), 46);
}

#[test]
fn neg16_truth_table() {
    let chip = Neg16::chip();
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
//     assert_eq!(flatten(Neg16::chip()).len(), 0);
// }

#[test]
fn alu_truth_table() {
    let chip = Alu::chip();

    // 0 = 0 + 0
    let r = eval(&chip, [("x", 0), ("y", 0), ("zx", 1), ("nx", 0), ("zy", 1), ("ny", 0), ("f", 1), ("no", 0)]);
    assert_eq!(r["out"], 0);      assert_eq!(r["zr"], 1); assert_eq!(r["ng"], 0); // 0

    // 1 = !(-1 + -1)
    let r = eval(&chip, [("x", 0), ("y", 0), ("zx", 1), ("nx", 1), ("zy", 1), ("ny", 1), ("f", 1), ("no", 1)]);
    assert_eq!(r["out"], 1);      assert_eq!(r["zr"], 0); assert_eq!(r["ng"], 0); // 1

    // -1 = -1 + 0
    let r = eval(&chip, [("x", 0), ("y", 0), ("zx", 1), ("nx", 1), ("zy", 1), ("ny", 0), ("f", 1), ("no", 0)]);
    assert_eq!(r["out"], 0xffff); assert_eq!(r["zr"], 0); assert_eq!(r["ng"], 1); // -1

    // x = x and 0xfff
    let r = eval(&chip, [("x", 5), ("y", 3), ("zx", 0), ("nx", 0), ("zy", 1), ("ny", 1), ("f", 0), ("no", 0)]);
    assert_eq!(r["out"], 5);      assert_eq!(r["zr"], 0); assert_eq!(r["ng"], 0); // x

    // y = 0xfff and y
    let r = eval(&chip, [("x", 5), ("y", 3), ("zx", 1), ("nx", 1), ("zy", 0), ("ny", 0), ("f", 0), ("no", 0)]);
    assert_eq!(r["out"], 3);      assert_eq!(r["zr"], 0); assert_eq!(r["ng"], 0); // y

    // x + y
    let r = eval(&chip, [("x", 5), ("y", 3), ("zx", 0), ("nx", 0), ("zy", 0), ("ny", 0), ("f", 1), ("no", 0)]);
    assert_eq!(r["out"], 8);      assert_eq!(r["zr"], 0); assert_eq!(r["ng"], 0); // x + y

    // x - y = !(!x + y)
    let r = eval(&chip, [("x", 5), ("y", 3), ("zx", 0), ("nx", 1), ("zy", 0), ("ny", 0), ("f", 1), ("no", 1)]);
    assert_eq!(r["out"], 2);      assert_eq!(r["zr"], 0); assert_eq!(r["ng"], 0); // x - y

    // x and y
    let r = eval(&chip, [("x", 0b1010), ("y", 0b1100), ("zx", 0), ("nx", 0), ("zy", 0), ("ny", 0), ("f", 0), ("no", 0)]);
    assert_eq!(r["out"], 0b1000); assert_eq!(r["zr"], 0); assert_eq!(r["ng"], 0); // x AND y

    // x or y = !(!x and !y)
    let r = eval(&chip, [("x", 0b1010), ("y", 0b0101), ("zx", 0), ("nx", 1), ("zy", 0), ("ny", 1), ("f", 0), ("no", 1)]);
    assert_eq!(r["out"], 0b1111); assert_eq!(r["zr"], 0); assert_eq!(r["ng"], 0); // x OR y
}

#[test]
fn alu_optimal() {
    // TODO: 560, once Neg16 is reduced to wiring only
    assert_eq!(flatten(Alu::chip()).len(), 562);
}
