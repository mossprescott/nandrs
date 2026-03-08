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
    let chip = And { a: Input::new(), b: Input::new(), out: Output::new() };
    assert_eq!(
        print_graph(&chip),
        "And:\n\
         nand0.a <- a\n\
         nand0.b <- b\n\
         not1.a <- nand0.out\n\
         out <- not1.out"
    );
}

#[test]
fn mux16_graph() {
    let chip = Mux16 { a0: Input16::new(), a1: Input16::new(), sel: Input::new(), out: Output16::new() };
    assert_eq!(
        print_graph(&chip),
        "Mux16:\n\
         not0.a <- sel\n\
         nand1.a <- not0.out\n\
         nand1.b <- a0[0]\n\
         nand2.a <- sel\n\
         nand2.b <- a1[0]\n\
         nand3.a <- nand1.out\n\
         nand3.b <- nand2.out\n\
         nand4.a <- not0.out\n\
         nand4.b <- a0[1]\n\
         nand5.a <- sel\n\
         nand5.b <- a1[1]\n\
         nand6.a <- nand4.out\n\
         nand6.b <- nand5.out\n\
         nand7.a <- not0.out\n\
         nand7.b <- a0[2]\n\
         nand8.a <- sel\n\
         nand8.b <- a1[2]\n\
         nand9.a <- nand7.out\n\
         nand9.b <- nand8.out\n\
         nand10.a <- not0.out\n\
         nand10.b <- a0[3]\n\
         nand11.a <- sel\n\
         nand11.b <- a1[3]\n\
         nand12.a <- nand10.out\n\
         nand12.b <- nand11.out\n\
         nand13.a <- not0.out\n\
         nand13.b <- a0[4]\n\
         nand14.a <- sel\n\
         nand14.b <- a1[4]\n\
         nand15.a <- nand13.out\n\
         nand15.b <- nand14.out\n\
         nand16.a <- not0.out\n\
         nand16.b <- a0[5]\n\
         nand17.a <- sel\n\
         nand17.b <- a1[5]\n\
         nand18.a <- nand16.out\n\
         nand18.b <- nand17.out\n\
         nand19.a <- not0.out\n\
         nand19.b <- a0[6]\n\
         nand20.a <- sel\n\
         nand20.b <- a1[6]\n\
         nand21.a <- nand19.out\n\
         nand21.b <- nand20.out\n\
         nand22.a <- not0.out\n\
         nand22.b <- a0[7]\n\
         nand23.a <- sel\n\
         nand23.b <- a1[7]\n\
         nand24.a <- nand22.out\n\
         nand24.b <- nand23.out\n\
         nand25.a <- not0.out\n\
         nand25.b <- a0[8]\n\
         nand26.a <- sel\n\
         nand26.b <- a1[8]\n\
         nand27.a <- nand25.out\n\
         nand27.b <- nand26.out\n\
         nand28.a <- not0.out\n\
         nand28.b <- a0[9]\n\
         nand29.a <- sel\n\
         nand29.b <- a1[9]\n\
         nand30.a <- nand28.out\n\
         nand30.b <- nand29.out\n\
         nand31.a <- not0.out\n\
         nand31.b <- a0[10]\n\
         nand32.a <- sel\n\
         nand32.b <- a1[10]\n\
         nand33.a <- nand31.out\n\
         nand33.b <- nand32.out\n\
         nand34.a <- not0.out\n\
         nand34.b <- a0[11]\n\
         nand35.a <- sel\n\
         nand35.b <- a1[11]\n\
         nand36.a <- nand34.out\n\
         nand36.b <- nand35.out\n\
         nand37.a <- not0.out\n\
         nand37.b <- a0[12]\n\
         nand38.a <- sel\n\
         nand38.b <- a1[12]\n\
         nand39.a <- nand37.out\n\
         nand39.b <- nand38.out\n\
         nand40.a <- not0.out\n\
         nand40.b <- a0[13]\n\
         nand41.a <- sel\n\
         nand41.b <- a1[13]\n\
         nand42.a <- nand40.out\n\
         nand42.b <- nand41.out\n\
         nand43.a <- not0.out\n\
         nand43.b <- a0[14]\n\
         nand44.a <- sel\n\
         nand44.b <- a1[14]\n\
         nand45.a <- nand43.out\n\
         nand45.b <- nand44.out\n\
         nand46.a <- not0.out\n\
         nand46.b <- a0[15]\n\
         nand47.a <- sel\n\
         nand47.b <- a1[15]\n\
         nand48.a <- nand46.out\n\
         nand48.b <- nand47.out\n\
         out[0] <- nand3.out\n\
         out[1] <- nand6.out\n\
         out[2] <- nand9.out\n\
         out[3] <- nand12.out\n\
         out[4] <- nand15.out\n\
         out[5] <- nand18.out\n\
         out[6] <- nand21.out\n\
         out[7] <- nand24.out\n\
         out[8] <- nand27.out\n\
         out[9] <- nand30.out\n\
         out[10] <- nand33.out\n\
         out[11] <- nand36.out\n\
         out[12] <- nand39.out\n\
         out[13] <- nand42.out\n\
         out[14] <- nand45.out\n\
         out[15] <- nand48.out"
    );
}
