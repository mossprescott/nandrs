use crate::project_02::{ALU, Add16, FullAdder, HalfAdder, Inc16, Neg16, Zero16, flatten_t};
use simulator::component::{Combinational, count_combinational};
use simulator::eval::eval;
use simulator::nat::{N1, N16};
use simulator::word::Word;
use simulator::{Chip as _, IC, print_graph, print_ic_graph};
use std::collections::HashMap;

fn eval1<'a>(
    chip: &IC<Combinational>,
    inputs: impl IntoIterator<Item = (&'a str, Word<N1>)>,
) -> HashMap<String, Word<N1>> {
    eval(chip, inputs)
}

fn eval16<'a>(
    chip: &IC<Combinational>,
    inputs: impl IntoIterator<Item = (&'a str, Word<N16>)>,
) -> HashMap<String, Word<N16>> {
    eval(chip, inputs)
}

#[test]
fn half_adder_truth_table() {
    let chip = flatten_t(HalfAdder::chip());

    println!("{}", print_ic_graph(&chip));

    let r = eval1(&chip, [("a", false.into()), ("b", false.into())]);
    assert_eq!(r["sum"].unsigned(), 0);
    assert_eq!(r["carry"].unsigned(), 0);
    let r = eval1(&chip, [("a", false.into()), ("b", true.into())]);
    assert_eq!(r["sum"].unsigned(), 1);
    assert_eq!(r["carry"].unsigned(), 0);
    let r = eval1(&chip, [("a", true.into()), ("b", false.into())]);
    assert_eq!(r["sum"].unsigned(), 1);
    assert_eq!(r["carry"].unsigned(), 0);
    let r = eval1(&chip, [("a", true.into()), ("b", true.into())]);
    assert_eq!(r["sum"].unsigned(), 0);
    assert_eq!(r["carry"].unsigned(), 1);
}

#[test]
fn half_adder_optimal() {
    let chip = flatten_t(HalfAdder::chip());
    assert_eq!(count_combinational(&chip.components).nands, 5);
}

#[test]
fn full_adder_truth_table() {
    let chip = flatten_t(FullAdder::chip());

    println!("{}", print_ic_graph(&chip));

    let r = eval1(
        &chip,
        [
            ("a", false.into()),
            ("b", false.into()),
            ("c", false.into()),
        ],
    );
    assert_eq!(r["sum"].unsigned(), 0);
    assert_eq!(r["carry"].unsigned(), 0);
    let r = eval1(
        &chip,
        [("a", false.into()), ("b", false.into()), ("c", true.into())],
    );
    assert_eq!(r["sum"].unsigned(), 1);
    assert_eq!(r["carry"].unsigned(), 0);
    let r = eval1(
        &chip,
        [("a", false.into()), ("b", true.into()), ("c", false.into())],
    );
    assert_eq!(r["sum"].unsigned(), 1);
    assert_eq!(r["carry"].unsigned(), 0);
    let r = eval1(
        &chip,
        [("a", false.into()), ("b", true.into()), ("c", true.into())],
    );
    assert_eq!(r["sum"].unsigned(), 0);
    assert_eq!(r["carry"].unsigned(), 1);
    let r = eval1(
        &chip,
        [("a", true.into()), ("b", false.into()), ("c", false.into())],
    );
    assert_eq!(r["sum"].unsigned(), 1);
    assert_eq!(r["carry"].unsigned(), 0);
    let r = eval1(
        &chip,
        [("a", true.into()), ("b", false.into()), ("c", true.into())],
    );
    assert_eq!(r["sum"].unsigned(), 0);
    assert_eq!(r["carry"].unsigned(), 1);
    let r = eval1(
        &chip,
        [("a", true.into()), ("b", true.into()), ("c", false.into())],
    );
    assert_eq!(r["sum"].unsigned(), 0);
    assert_eq!(r["carry"].unsigned(), 1);
    let r = eval1(
        &chip,
        [("a", true.into()), ("b", true.into()), ("c", true.into())],
    );
    assert_eq!(r["sum"].unsigned(), 1);
    assert_eq!(r["carry"].unsigned(), 1);
}

#[test]
fn full_adder_optimal() {
    let chip = flatten_t(FullAdder::chip());
    assert_eq!(count_combinational(&chip.components).nands, 9);
}

#[test]
fn inc16_truth_table() {
    let chip = Inc16::chip();

    // When it breaks, it's nice to see what it tried to do
    print!("{}", print_graph(&chip));

    let chip = flatten_t(chip);

    assert_eq!(eval16(&chip, [("a", 0u16.into())])["out"].unsigned(), 1);
    assert_eq!(eval16(&chip, [("a", 1u16.into())])["out"].unsigned(), 2);
    assert_eq!(eval16(&chip, [("a", 42u16.into())])["out"].unsigned(), 43);
    assert_eq!(
        eval16(&chip, [("a", 0xFFFFu16.into())])["out"].unsigned(),
        0
    ); // overflow wraps
}

#[test]
fn inc16_optimal() {
    let chip = flatten_t(Inc16::chip());
    // Not(1) for bit 0, plus 15 HalfAdders × 5 nands each for the carry chain
    assert_eq!(count_combinational(&chip.components).nands, 1 + 15 * 5);
}

#[test]
fn add16_truth_table() {
    let chip = Add16::chip();

    // When it breaks, it's nice to see what it tried to do
    print!("{}", print_graph(&chip));

    let chip = flatten_t(chip);

    assert_eq!(
        eval16(&chip, [("a", 0u16.into()), ("b", 0u16.into())])["out"].signed(),
        0
    );
    assert_eq!(
        eval16(&chip, [("a", 1u16.into()), ("b", 1u16.into())])["out"].signed(),
        2
    );
    assert_eq!(
        eval16(&chip, [("a", 100u16.into()), ("b", 200u16.into())])["out"].signed(),
        300
    );
    assert_eq!(
        eval16(&chip, [("a", 0xFFFFu16.into()), ("b", 1u16.into())])["out"].signed(),
        0
    ); // overflow wraps

    assert_eq!(
        eval16(&chip, [("a", (-1i16).into()), ("b", (-2i16).into())])["out"].signed(),
        -3
    );
    assert_eq!(
        eval16(&chip, [("a", (-32768i16).into()), ("b", (-1i16).into())])["out"].signed(),
        32767
    );
}

#[test]
fn add16_optimal() {
    let chip = flatten_t(Add16::chip());
    // 16 FullAdders × 9 nands each
    assert_eq!(count_combinational(&chip.components).nands, 16 * 9);
}

#[test]
fn zero16_truth_table() {
    let chip = Zero16::chip();

    // When it breaks, it's nice to see what it tried to do
    print!("{}", print_graph(&chip));

    let chip = flatten_t(chip);

    assert_eq!(eval16(&chip, [("a", 0u16.into())])["out"].unsigned(), 1); // all zeros
    assert_eq!(eval16(&chip, [("a", 1u16.into())])["out"].unsigned(), 0); // bit 0 set
    assert_eq!(
        eval16(&chip, [("a", 0x8000u16.into())])["out"].unsigned(),
        0
    ); // only MSB set
    assert_eq!(
        eval16(&chip, [("a", 0xFFFFu16.into())])["out"].unsigned(),
        0
    ); // all ones
}

#[test]
fn zero16_optimal() {
    let chip = flatten_t(Zero16::chip());
    // negate each bit: 16
    // and-tree: 2*(8+4+2+1) = 30
    // two nots because we use 16-way nand for "realism"
    assert_eq!(count_combinational(&chip.components).nands, 48);
}

#[test]
fn neg16_truth_table() {
    let chip = Neg16::chip();

    // When it breaks, it's nice to see what it tried to do
    print!("{}", print_graph(&chip));

    let chip = flatten_t(chip);

    assert_eq!(eval16(&chip, [("a", 0u16.into())])["out"].unsigned(), 0); // zero is not negative
    assert_eq!(eval16(&chip, [("a", 1u16.into())])["out"].unsigned(), 0); // positive
    assert_eq!(
        eval16(&chip, [("a", 0x7FFFu16.into())])["out"].unsigned(),
        0
    ); // max positive
    assert_eq!(
        eval16(&chip, [("a", 0x8000u16.into())])["out"].unsigned(),
        1
    ); // min negative (-32768)
    assert_eq!(
        eval16(&chip, [("a", 0xFFFFu16.into())])["out"].unsigned(),
        1
    ); // -1
}

#[test]
fn neg16_optimal() {
    let chip = flatten_t(Neg16::chip());
    assert_eq!(count_combinational(&chip.components).nands, 0);
}

#[test]
fn alu_truth_table() {
    let chip = ALU::chip();

    // When it breaks, it's nice to see what it tried to do
    print!("{}", print_graph(&chip));

    let chip = flatten_t(chip);

    // 0 = 0 + 0
    let r = eval16(
        &chip,
        [
            ("x", 0u16.into()),
            ("y", 0u16.into()),
            ("zx", true.into()),
            ("nx", false.into()),
            ("zy", true.into()),
            ("ny", false.into()),
            ("f", true.into()),
            ("no", false.into()),
        ],
    );
    assert_eq!(r["out"].unsigned(), 0);
    assert_eq!(r["zr"].unsigned(), 1);
    assert_eq!(r["ng"].unsigned(), 0); // 0

    // 1 = !(-1 + -1)
    let r = eval16(
        &chip,
        [
            ("x", 0u16.into()),
            ("y", 0u16.into()),
            ("zx", true.into()),
            ("nx", true.into()),
            ("zy", true.into()),
            ("ny", true.into()),
            ("f", true.into()),
            ("no", true.into()),
        ],
    );
    assert_eq!(r["out"].unsigned(), 1);
    assert_eq!(r["zr"].unsigned(), 0);
    assert_eq!(r["ng"].unsigned(), 0); // 1

    // -1 = -1 + 0
    let r = eval16(
        &chip,
        [
            ("x", 0u16.into()),
            ("y", 0u16.into()),
            ("zx", true.into()),
            ("nx", true.into()),
            ("zy", true.into()),
            ("ny", false.into()),
            ("f", true.into()),
            ("no", false.into()),
        ],
    );
    assert_eq!(r["out"].unsigned(), 0xffff);
    assert_eq!(r["zr"].unsigned(), 0);
    assert_eq!(r["ng"].unsigned(), 1); // -1

    // x = x and 0xfff
    let r = eval16(
        &chip,
        [
            ("x", 5u16.into()),
            ("y", 3u16.into()),
            ("zx", false.into()),
            ("nx", false.into()),
            ("zy", true.into()),
            ("ny", true.into()),
            ("f", false.into()),
            ("no", false.into()),
        ],
    );
    assert_eq!(r["out"].unsigned(), 5);
    assert_eq!(r["zr"].unsigned(), 0);
    assert_eq!(r["ng"].unsigned(), 0); // x

    // y = 0xfff and y
    let r = eval16(
        &chip,
        [
            ("x", 5u16.into()),
            ("y", 3u16.into()),
            ("zx", true.into()),
            ("nx", true.into()),
            ("zy", false.into()),
            ("ny", false.into()),
            ("f", false.into()),
            ("no", false.into()),
        ],
    );
    assert_eq!(r["out"].unsigned(), 3);
    assert_eq!(r["zr"].unsigned(), 0);
    assert_eq!(r["ng"].unsigned(), 0); // y

    // x + y
    let r = eval16(
        &chip,
        [
            ("x", 5u16.into()),
            ("y", 3u16.into()),
            ("zx", false.into()),
            ("nx", false.into()),
            ("zy", false.into()),
            ("ny", false.into()),
            ("f", true.into()),
            ("no", false.into()),
        ],
    );
    assert_eq!(r["out"].unsigned(), 8);
    assert_eq!(r["zr"].unsigned(), 0);
    assert_eq!(r["ng"].unsigned(), 0); // x + y

    // x - y = !(!x + y)
    let r = eval16(
        &chip,
        [
            ("x", 5u16.into()),
            ("y", 3u16.into()),
            ("zx", false.into()),
            ("nx", true.into()),
            ("zy", false.into()),
            ("ny", false.into()),
            ("f", true.into()),
            ("no", true.into()),
        ],
    );
    assert_eq!(r["out"].unsigned(), 2);
    assert_eq!(r["zr"].unsigned(), 0);
    assert_eq!(r["ng"].unsigned(), 0); // x - y

    // x and y
    let r = eval16(
        &chip,
        [
            ("x", 0b1010u16.into()),
            ("y", 0b1100u16.into()),
            ("zx", false.into()),
            ("nx", false.into()),
            ("zy", false.into()),
            ("ny", false.into()),
            ("f", false.into()),
            ("no", false.into()),
        ],
    );
    assert_eq!(r["out"].unsigned(), 0b1000);
    assert_eq!(r["zr"].unsigned(), 0);
    assert_eq!(r["ng"].unsigned(), 0); // x AND y

    // x or y = !(!x and !y)
    let r = eval16(
        &chip,
        [
            ("x", 0b1010u16.into()),
            ("y", 0b0101u16.into()),
            ("zx", false.into()),
            ("nx", true.into()),
            ("zy", false.into()),
            ("ny", true.into()),
            ("f", false.into()),
            ("no", true.into()),
        ],
    );
    assert_eq!(r["out"].unsigned(), 0b1111);
    assert_eq!(r["zr"].unsigned(), 0);
    assert_eq!(r["ng"].unsigned(), 0); // x OR y
}

#[test]
fn alu_optimal() {
    let chip = flatten_t(ALU::chip());
    assert_eq!(count_combinational(&chip.components).nands, 720);
}

#[test]
fn alu_graph() {
    let chip = ALU::chip();
    assert_eq!(
        print_graph(&chip),
        "ALU:
  mux16_0.a0[0..15] <- x[0..15]
  mux16_0.a1[0..15] <- 0
  mux16_0.sel <- zx
  not16_1.a[0..15] <- mux16_0.out[0..15]
  mux16_2.a0[0..15] <- mux16_0.out[0..15]
  mux16_2.a1[0..15] <- not16_1.out[0..15]
  mux16_2.sel <- nx
  mux16_3.a0[0..15] <- y[0..15]
  mux16_3.a1[0..15] <- 0
  mux16_3.sel <- zy
  not16_4.a[0..15] <- mux16_3.out[0..15]
  mux16_5.a0[0..15] <- mux16_3.out[0..15]
  mux16_5.a1[0..15] <- not16_4.out[0..15]
  mux16_5.sel <- ny
  and16_6.a[0..15] <- mux16_2.out[0..15]
  and16_6.b[0..15] <- mux16_5.out[0..15]
  not_7.a <- disable
  and_8.a <- f
  and_8.b <- not_7.out
  mux16_9.a0[0..15] <- 0
  mux16_9.a1[0..15] <- mux16_2.out[0..15]
  mux16_9.sel <- and_8.out
  mux16_10.a0[0..15] <- 0
  mux16_10.a1[0..15] <- mux16_5.out[0..15]
  mux16_10.sel <- and_8.out
  add16_11.a[0..15] <- mux16_9.out[0..15]
  add16_11.b[0..15] <- mux16_10.out[0..15]
  mux16_12.a0[0..15] <- and16_6.out[0..15]
  mux16_12.a1[0..15] <- add16_11.out[0..15]
  mux16_12.sel <- f
  not16_13.a[0..15] <- mux16_12.out[0..15]
  mux16_14.a0[0..15] <- mux16_12.out[0..15]
  mux16_14.a1[0..15] <- not16_13.out[0..15]
  mux16_14.sel <- no
  zero16_15.a[0..15] <- mux16_14.out[0..15]
  mux16_16.a0[0..15] <- mux16_14.out[0..15]
  mux16_16.a1[0..15] <- 0
  mux16_16.sel <- disable
  mux_17.a0 <- zero16_15.out
  mux_17.a1 <- 1
  mux_17.sel <- disable
  neg16_18.a[0..15] <- mux16_16.out[0..15]
  ng <- neg16_18.out
  out[0..15] <- mux16_16.out[0..15]
  zr <- mux_17.out"
    );
}
