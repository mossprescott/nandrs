// use simulator::nand;
use simulator::Component;
use simulator::ConnectionWidth::Wire;
use crate::project_01::*;

#[test]
fn nand_inputs() {
    let inputs = Nand.inputs();
    assert_eq!(inputs.len(), 2);
    assert_eq!(inputs["a"], Wire);
    assert_eq!(inputs["b"], Wire);

    let outputs = Nand.outputs();
    assert_eq!(outputs.len(), 1);
    assert_eq!(outputs["out"], Wire);
}


// #[test]
// fn nand_truth_table() {
//     // let a =
//     // // let chip = nand()
//     // assert_eq!(nand(false, false), true);
//     // assert_eq!(nand(false, true),  true);
//     // assert_eq!(nand(true,  false), true);
//     // assert_eq!(nand(true,  true),  false);
// }

// #[test]
// fn not_truth_table() {
//     let a_in = Wire {}
//     let b_in = Wire {}
//     let chip = Not { a = a_in, b = b_in }.build()

//     assert_eq!(not(false), true);
//     assert_eq!(not(true),  false);
// }

// #[test]
// fn and_truth_table() {
//     assert_eq!(and(false, false), false);
//     assert_eq!(and(false, true),  false);
//     assert_eq!(and(true,  false), false);
//     assert_eq!(and(true,  true),  true);
// }

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
