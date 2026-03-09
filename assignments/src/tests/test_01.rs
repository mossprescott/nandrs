use simulator::{Input, Input16, Output, Output16, Reflect, print_graph};
use simulator::Chip as _;
use simulator::eval::eval;
use crate::project_01::{flatten, Nand, Not, And, Or, Xor, Mux, Dmux, Not16, And16, Mux16};

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
    assert_eq!(flatten(Mux::chip()).components.len(), 4);
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
    assert_eq!(flatten(Mux16::chip()).components.len(), 49);
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

#[test]
fn mux16_graph() {
    let chip = Mux16::chip();
    assert_eq!(
        print_graph(&chip),
        "Mux16:\n\
         not_0.a <- sel\n\
         nand_1.a <- not_0.out\n\
         nand_1.b <- a0[0]\n\
         nand_2.a <- sel\n\
         nand_2.b <- a1[0]\n\
         nand_3.a <- nand_1.out\n\
         nand_3.b <- nand_2.out\n\
         nand_4.a <- not_0.out\n\
         nand_4.b <- a0[1]\n\
         nand_5.a <- sel\n\
         nand_5.b <- a1[1]\n\
         nand_6.a <- nand_4.out\n\
         nand_6.b <- nand_5.out\n\
         nand_7.a <- not_0.out\n\
         nand_7.b <- a0[2]\n\
         nand_8.a <- sel\n\
         nand_8.b <- a1[2]\n\
         nand_9.a <- nand_7.out\n\
         nand_9.b <- nand_8.out\n\
         nand_10.a <- not_0.out\n\
         nand_10.b <- a0[3]\n\
         nand_11.a <- sel\n\
         nand_11.b <- a1[3]\n\
         nand_12.a <- nand_10.out\n\
         nand_12.b <- nand_11.out\n\
         nand_13.a <- not_0.out\n\
         nand_13.b <- a0[4]\n\
         nand_14.a <- sel\n\
         nand_14.b <- a1[4]\n\
         nand_15.a <- nand_13.out\n\
         nand_15.b <- nand_14.out\n\
         nand_16.a <- not_0.out\n\
         nand_16.b <- a0[5]\n\
         nand_17.a <- sel\n\
         nand_17.b <- a1[5]\n\
         nand_18.a <- nand_16.out\n\
         nand_18.b <- nand_17.out\n\
         nand_19.a <- not_0.out\n\
         nand_19.b <- a0[6]\n\
         nand_20.a <- sel\n\
         nand_20.b <- a1[6]\n\
         nand_21.a <- nand_19.out\n\
         nand_21.b <- nand_20.out\n\
         nand_22.a <- not_0.out\n\
         nand_22.b <- a0[7]\n\
         nand_23.a <- sel\n\
         nand_23.b <- a1[7]\n\
         nand_24.a <- nand_22.out\n\
         nand_24.b <- nand_23.out\n\
         nand_25.a <- not_0.out\n\
         nand_25.b <- a0[8]\n\
         nand_26.a <- sel\n\
         nand_26.b <- a1[8]\n\
         nand_27.a <- nand_25.out\n\
         nand_27.b <- nand_26.out\n\
         nand_28.a <- not_0.out\n\
         nand_28.b <- a0[9]\n\
         nand_29.a <- sel\n\
         nand_29.b <- a1[9]\n\
         nand_30.a <- nand_28.out\n\
         nand_30.b <- nand_29.out\n\
         nand_31.a <- not_0.out\n\
         nand_31.b <- a0[10]\n\
         nand_32.a <- sel\n\
         nand_32.b <- a1[10]\n\
         nand_33.a <- nand_31.out\n\
         nand_33.b <- nand_32.out\n\
         nand_34.a <- not_0.out\n\
         nand_34.b <- a0[11]\n\
         nand_35.a <- sel\n\
         nand_35.b <- a1[11]\n\
         nand_36.a <- nand_34.out\n\
         nand_36.b <- nand_35.out\n\
         nand_37.a <- not_0.out\n\
         nand_37.b <- a0[12]\n\
         nand_38.a <- sel\n\
         nand_38.b <- a1[12]\n\
         nand_39.a <- nand_37.out\n\
         nand_39.b <- nand_38.out\n\
         nand_40.a <- not_0.out\n\
         nand_40.b <- a0[13]\n\
         nand_41.a <- sel\n\
         nand_41.b <- a1[13]\n\
         nand_42.a <- nand_40.out\n\
         nand_42.b <- nand_41.out\n\
         nand_43.a <- not_0.out\n\
         nand_43.b <- a0[14]\n\
         nand_44.a <- sel\n\
         nand_44.b <- a1[14]\n\
         nand_45.a <- nand_43.out\n\
         nand_45.b <- nand_44.out\n\
         nand_46.a <- not_0.out\n\
         nand_46.b <- a0[15]\n\
         nand_47.a <- sel\n\
         nand_47.b <- a1[15]\n\
         nand_48.a <- nand_46.out\n\
         nand_48.b <- nand_47.out\n\
         out[0] <- nand_3.out\n\
         out[1] <- nand_6.out\n\
         out[2] <- nand_9.out\n\
         out[3] <- nand_12.out\n\
         out[4] <- nand_15.out\n\
         out[5] <- nand_18.out\n\
         out[6] <- nand_21.out\n\
         out[7] <- nand_24.out\n\
         out[8] <- nand_27.out\n\
         out[9] <- nand_30.out\n\
         out[10] <- nand_33.out\n\
         out[11] <- nand_36.out\n\
         out[12] <- nand_39.out\n\
         out[13] <- nand_42.out\n\
         out[14] <- nand_45.out\n\
         out[15] <- nand_48.out"
    );
}
