use crate::project_01::{
    And, And16, Dmux, Mux, Mux16, Nand, Not, Not16, Or, Xor, flatten, flatten_t,
};
use simulator::Chip as _;
use simulator::component::{Combinational, CombinationalT, count_combinational};
use simulator::eval::{eval, eval_t};
use simulator::nat::{N1, N16};
use simulator::word::Word;
use simulator::{Component, IC, Input1, Output, Reflect, print_graph};
use std::collections::HashMap;

fn eval1<'a>(
    chip: &IC<Combinational>,
    inputs: impl IntoIterator<Item = (&'a str, Word<N1>)>,
) -> HashMap<String, Word<N1>> {
    let chip: IC<CombinationalT> = chip.map(Into::into);
    eval_t(&chip, inputs)
}

fn eval16<'a>(
    chip: &IC<Combinational>,
    inputs: impl IntoIterator<Item = (&'a str, Word<N16>)>,
) -> HashMap<String, Word<N16>> {
    let chip: IC<CombinationalT> = chip.map(Into::into);
    eval_t(&chip, inputs)
}

fn eval1_t<'a>(
    chip: &IC<CombinationalT>,
    inputs: impl IntoIterator<Item = (&'a str, Word<N1>)>,
) -> HashMap<String, Word<N1>> {
    eval_t(&chip, inputs)
}

#[test]
fn nand_reflect() {
    let chip = Nand {
        a: Input1::new(),
        b: Input1::new(),
        out: Output::new(),
    };
    let intf = chip.reflect();
    assert_eq!(intf.inputs.len(), 2);
    assert_eq!(intf.inputs["a"].width, 1);
    assert_eq!(intf.inputs["b"].width, 1);
    assert_eq!(intf.outputs.len(), 1);
    assert_eq!(intf.outputs["out"].width, 1);
}

#[test]
fn nand_truth_table() {
    let chip = flatten_t(Nand::chip());
    assert_eq!(
        eval1_t(&chip, [("a", false.into()), ("b", false.into())])["out"].unsigned(),
        1
    );
    assert_eq!(
        eval1_t(&chip, [("a", false.into()), ("b", true.into())])["out"].unsigned(),
        1
    );
    assert_eq!(
        eval1_t(&chip, [("a", true.into()), ("b", false.into())])["out"].unsigned(),
        1
    );
    assert_eq!(
        eval1_t(&chip, [("a", true.into()), ("b", true.into())])["out"].unsigned(),
        0
    );
}

#[test]
fn not_truth_table() {
    let chip = flatten_t(Not::chip());
    assert_eq!(eval1_t(&chip, [("a", false.into())])["out"].unsigned(), 1);
    assert_eq!(eval1_t(&chip, [("a", true.into())])["out"].unsigned(), 0);
}

#[test]
fn not_optimal() {
    let chip = flatten(Not::chip());
    assert_eq!(count_combinational(&chip.components).nands, 1);
}

#[test]
fn and_truth_table() {
    let chip = flatten_t(And::chip());
    assert_eq!(
        eval1_t(&chip, [("a", false.into()), ("b", false.into())])["out"].unsigned(),
        0
    );
    assert_eq!(
        eval1_t(&chip, [("a", false.into()), ("b", true.into())])["out"].unsigned(),
        0
    );
    assert_eq!(
        eval1_t(&chip, [("a", true.into()), ("b", false.into())])["out"].unsigned(),
        0
    );
    assert_eq!(
        eval1_t(&chip, [("a", true.into()), ("b", true.into())])["out"].unsigned(),
        1
    );
}

#[test]
fn and_optimal() {
    let chip = flatten(And::chip());
    assert_eq!(count_combinational(&chip.components).nands, 2);
}

#[test]
fn or_truth_table() {
    let chip = flatten_t(Or::chip());
    assert_eq!(
        eval1_t(&chip, [("a", false.into()), ("b", false.into())])["out"].unsigned(),
        0
    );
    assert_eq!(
        eval1_t(&chip, [("a", false.into()), ("b", true.into())])["out"].unsigned(),
        1
    );
    assert_eq!(
        eval1_t(&chip, [("a", true.into()), ("b", false.into())])["out"].unsigned(),
        1
    );
    assert_eq!(
        eval1_t(&chip, [("a", true.into()), ("b", true.into())])["out"].unsigned(),
        1
    );
}

#[test]
fn or_optimal() {
    let chip = flatten(Or::chip());
    assert_eq!(count_combinational(&chip.components).nands, 3);
}

#[test]
fn xor_truth_table() {
    let chip = flatten_t(Xor::chip());
    assert_eq!(
        eval1_t(&chip, [("a", false.into()), ("b", false.into())])["out"].unsigned(),
        0
    );
    assert_eq!(
        eval1_t(&chip, [("a", false.into()), ("b", true.into())])["out"].unsigned(),
        1
    );
    assert_eq!(
        eval1_t(&chip, [("a", true.into()), ("b", false.into())])["out"].unsigned(),
        1
    );
    assert_eq!(
        eval1_t(&chip, [("a", true.into()), ("b", true.into())])["out"].unsigned(),
        0
    );
}

#[test]
fn xor_optimal() {
    let chip = flatten(Xor::chip());
    assert_eq!(count_combinational(&chip.components).nands, 4);
}

#[test]
fn mux_truth_table() {
    let chip = flatten_t(Mux::chip());
    assert_eq!(
        eval1_t(
            &chip,
            [
                ("a0", false.into()),
                ("a1", false.into()),
                ("sel", false.into())
            ]
        )["out"]
            .unsigned(),
        0
    );
    assert_eq!(
        eval1_t(
            &chip,
            [
                ("a0", false.into()),
                ("a1", true.into()),
                ("sel", false.into())
            ]
        )["out"]
            .unsigned(),
        0
    );
    assert_eq!(
        eval1_t(
            &chip,
            [
                ("a0", true.into()),
                ("a1", false.into()),
                ("sel", false.into())
            ]
        )["out"]
            .unsigned(),
        1
    );
    assert_eq!(
        eval1_t(
            &chip,
            [
                ("a0", true.into()),
                ("a1", true.into()),
                ("sel", false.into())
            ]
        )["out"]
            .unsigned(),
        1
    );
    assert_eq!(
        eval1_t(
            &chip,
            [
                ("a0", false.into()),
                ("a1", false.into()),
                ("sel", true.into())
            ]
        )["out"]
            .unsigned(),
        0
    );
    assert_eq!(
        eval1_t(
            &chip,
            [
                ("a0", false.into()),
                ("a1", true.into()),
                ("sel", true.into())
            ]
        )["out"]
            .unsigned(),
        1
    );
    assert_eq!(
        eval1_t(
            &chip,
            [
                ("a0", true.into()),
                ("a1", false.into()),
                ("sel", true.into())
            ]
        )["out"]
            .unsigned(),
        0
    );
    assert_eq!(
        eval1_t(
            &chip,
            [
                ("a0", true.into()),
                ("a1", true.into()),
                ("sel", true.into())
            ]
        )["out"]
            .unsigned(),
        1
    );
}

#[test]
fn mux_optimal() {
    let chip = flatten(Mux::chip());
    assert_eq!(count_combinational(&chip.components).nands, 4);
}

#[test]
fn dmux_truth_table() {
    let chip = flatten_t(Dmux::chip());
    let r = eval1_t(&chip, [("input", false.into()), ("sel", false.into())]);
    assert_eq!((r["a"].unsigned(), r["b"].unsigned()), (0, 0));
    let r = eval1_t(&chip, [("input", true.into()), ("sel", false.into())]);
    assert_eq!((r["a"].unsigned(), r["b"].unsigned()), (1, 0));
    let r = eval1_t(&chip, [("input", false.into()), ("sel", true.into())]);
    assert_eq!((r["a"].unsigned(), r["b"].unsigned()), (0, 0));
    let r = eval1_t(&chip, [("input", true.into()), ("sel", true.into())]);
    assert_eq!((r["a"].unsigned(), r["b"].unsigned()), (0, 1));
}

#[test]
fn dmux_optimal() {
    let chip = flatten(Dmux::chip());
    assert_eq!(count_combinational(&chip.components).nands, 5);
}

#[test]
fn not16_truth_table() {
    let chip = flatten(Not16::chip());
    assert_eq!(
        eval16(&chip, [("a", 0x0000u16.into())])["out"].unsigned(),
        0xFFFF
    );
    assert_eq!(
        eval16(&chip, [("a", 0xFFFFu16.into())])["out"].unsigned(),
        0x0000
    );
    assert_eq!(
        eval16(&chip, [("a", 0xAAAAu16.into())])["out"].unsigned(),
        0x5555
    );
    assert_eq!(
        eval16(&chip, [("a", 0x1234u16.into())])["out"].unsigned(),
        0xEDCB
    );
}

#[test]
fn not16_optimal() {
    let chip = flatten(Not16::chip());
    assert_eq!(count_combinational(&chip.components).nands, 16);
}

#[test]
fn and16_truth_table() {
    let chip = flatten(And16::chip());
    assert_eq!(
        eval16(&chip, [("a", 0xFFFFu16.into()), ("b", 0xAAAAu16.into())])["out"].unsigned(),
        0xAAAA
    );
    assert_eq!(
        eval16(&chip, [("a", 0x0000u16.into()), ("b", 0xFFFFu16.into())])["out"].unsigned(),
        0x0000
    );
    assert_eq!(
        eval16(&chip, [("a", 0xFF00u16.into()), ("b", 0x0FF0u16.into())])["out"].unsigned(),
        0x0F00
    );
    assert_eq!(
        eval16(&chip, [("a", 0xFFFFu16.into()), ("b", 0xFFFFu16.into())])["out"].unsigned(),
        0xFFFF
    );
}

#[test]
fn and16_optimal() {
    let chip = flatten(And16::chip());
    assert_eq!(count_combinational(&chip.components).nands, 32);
}

// #[test]
// fn or16_truth_table() {
//     let chip = flatten(Or16::chip());
//     assert_eq!(eval16(&chip, [("a", 0x0000u16.into()), ("b", 0xAAAAu16.into())])["out"].unsigned(), 0xAAAA);
//     assert_eq!(eval16(&chip, [("a", 0x5555u16.into()), ("b", 0xAAAAu16.into())])["out"].unsigned(), 0xFFFF);
//     assert_eq!(eval16(&chip, [("a", 0xFF00u16.into()), ("b", 0x00FFu16.into())])["out"].unsigned(), 0xFFFF);
//     assert_eq!(eval16(&chip, [("a", 0x1234u16.into()), ("b", 0x0F0Fu16.into())])["out"].unsigned(), 0x1F3F);
// }

// #[test]
// fn or16_optimal() {
//     assert_eq!(flatten(Or16::chip()).components.len(), 48);
// }

/// Sanity check
#[test]
fn mux16_truth_table() {
    let chip = flatten(Mux16::chip());
    assert_eq!(
        eval16(
            &chip,
            [
                ("a0", 0xAAAAu16.into()),
                ("a1", 0x5555u16.into()),
                ("sel", 0u16.into())
            ]
        )["out"]
            .unsigned(),
        0xAAAA
    );
    assert_eq!(
        eval16(
            &chip,
            [
                ("a0", 0xAAAAu16.into()),
                ("a1", 0x5555u16.into()),
                ("sel", 1u16.into())
            ]
        )["out"]
            .unsigned(),
        0x5555
    );
    assert_eq!(
        eval16(
            &chip,
            [
                ("a0", 0x1234u16.into()),
                ("a1", 0x5678u16.into()),
                ("sel", 0u16.into())
            ]
        )["out"]
            .unsigned(),
        0x1234
    );
    assert_eq!(
        eval16(
            &chip,
            [
                ("a0", 0x1234u16.into()),
                ("a1", 0x5678u16.into()),
                ("sel", 1u16.into())
            ]
        )["out"]
            .unsigned(),
        0x5678
    );
}

#[test]
fn mux16_optimal() {
    let chip = flatten(Mux16::chip());
    assert_eq!(count_combinational(&chip.components).nands, 49);
}

#[test]
fn and_graph() {
    let chip = And::chip();
    assert_eq!(
        print_graph(&chip),
        "And:
  nand_0.a <- a
  nand_0.b <- b
  not_1.a <- nand_0.out
  out <- not_1.out"
    );
}
