/// Tests of the handling of important chips by the simulator.
///
/// Note: when these tests fail, the problem is in the `synthesize`, or maybe the components in
/// test_02 have been implemented in a new way that isn't handled yet.

use simulator::{Reflect, IC, Chip as _};
use simulator::component::Computational16;
use simulator::nat::N16;
use simulator::simulate::{MemoryMap, synthesize, ChipWiring};
use crate::project_02::{flatten, Inc16, Add16, Zero16};

fn synth<C: Reflect + Clone + Into<crate::project_02::Project02Component>>(chip: C) -> ChipWiring<N16> {
    let flat: IC<Computational16> = flatten(chip).map(Into::into);
    synthesize(&flat, MemoryMap::new(vec![]))
}

/// Test that the adder bit-slices in Add16 get coalesced into a single ripple-add operation.
#[test]
fn add16_wiring() {
    let wiring = synth(Add16::chip());
    println!("{wiring}");

    // Note: the details aren't exposed, so for now just a "black box" thumbs up or down on whether
    // they got optimized.
    let ops = wiring.op_counts();
    assert_eq!(ops.ripple_adders, 1, "16 adders should coalesce into 1 ripple adder");
    assert_eq!(ops.adders, 0, "no individual adders should remain");
}

/// Test that Inc16's carry chain gets coalesced despite per-bit fixed(0) b inputs.
///
/// And yes, this does feel a bit dirty. If somebody decides to build their inc in a slightly
/// different way, this just falls flat. For example,
#[test]
fn inc16_wiring() {
    let wiring = synth(Inc16::chip());
    println!("{wiring}");

    // Note: the details aren't exposed, so for now just a "black box" thumbs up or down on whether
    // they got optimized.
    let ops = wiring.op_counts();
    assert_eq!(ops.ripple_adders, 1, "16 adders should coalesce into 1 ripple adder");
    assert_eq!(ops.adders, 0, "no individual adders should remain");
}


/// Test that Zero16's Not16 and Nand16Way sequences both get squashed down to two fancy ops.
#[test]
fn zero16_wiring() {
    let wiring = synth(Zero16::chip());
    println!("{wiring}");

    // Note: the details aren't exposed, so for now just a "black box" thumbs up or down on whether
    // they got optimized.
    let ops = wiring.op_counts();
    assert_eq!(ops.parallel_nands, 1, "Not16 should coalesce into 1 parallel nand");
    assert_eq!(ops.many_way_ands, 1, "and-tree + final not should coalesce into 1 many-way and");
    assert_eq!(ops.nands, 0, "no individual nands should remain");
    assert_eq!(ops.ands, 0, "no individual ands should remain");
}