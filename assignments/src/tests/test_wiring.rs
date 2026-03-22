/// Tests of the handling of important chips by the simulator.

use simulator::{Reflect, IC, Chip as _};
use simulator::component::Computational16;
use simulator::nat::N16;
use simulator::simulate::{MemoryMap, synthesize, ChipWiring};
use crate::project_02::{flatten, Inc16, Add16};

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