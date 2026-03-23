/// A simple evaluator for "combinational" chips, consisting only of Nands.
///
/// There is no clock and no state.

use std::collections::HashMap;

use crate::component::Combinational;
use crate::declare::{BusRef, IC, Reflect};
use crate::nat::Nat;
use crate::word::{Storable, Word};

/// Evaluate a chip statelessly; given named input values, return named output values.
///
/// Input and output values are `Word<Width>`, wrapping the raw bits in a width-aware type.
pub fn eval<'a, Width: Nat + Clone, I>(chip: &IC<Combinational>, inputs: I) -> HashMap<String, Word<Width>>
where
    Width: Storable,
    I: IntoIterator<Item = (&'a str, Word<Width>)>,
{
    let intf = chip.reflect();

    // Map bus identity → value (all bits of the bus packed into a u64).
    let mut wire_state: HashMap<usize, u64> = HashMap::new();

    // Seed with the provided input values.
    for (name, value) in inputs {
        if let Some(busref) = intf.inputs.get(name) {
            let id = wire_id(busref);
            let mask = bus_mask(busref);
            let entry = wire_state.entry(id).or_insert(0);
            *entry = (*entry & !mask) | ((value.unsigned() << busref.offset) & mask);
        }
    }

    // Seed fixed (constant) inputs on each component.
    for comp in &chip.components {
        let comp_intf = comp.reflect();
        for busref in comp_intf.inputs.values() {
            if let Some(value) = busref.fixed {
                let id = wire_id(busref);
                let mask = bus_mask(busref);
                let entry = wire_state.entry(id).or_insert(0);
                *entry = (*entry & !mask) | ((value << busref.offset) & mask);
            }
        }
    }

    // Evaluate each component in order.
    for comp in &chip.components {
        match comp {
            Combinational::Nand(nand) => {
                let intf = nand.reflect();
                let a = read_bit(&wire_state, &intf.inputs["a"]);
                let b = read_bit(&wire_state, &intf.inputs["b"]);
                write_bit(&mut wire_state, &intf.outputs["out"],
                    !(a & b));
            }
            Combinational::Buffer(buffer) => {
                let intf = buffer.reflect();
                let a = read_bit(&wire_state, &intf.inputs["a"]);
                write_bit(&mut wire_state, &intf.outputs["out"], a);
            }
        }
    }

    // Read named outputs.
    intf.outputs
        .iter()
        .map(|(name, busref)| {
            let val = wire_state.get(&wire_id(busref)).copied().unwrap_or(0);
            (name.clone(), Word::new((val >> busref.offset) & width_mask(busref.width)))
        })
        .collect()
}

fn wire_id(busref: &BusRef) -> usize {
    busref.id.0
}

fn width_mask(width: usize) -> u64 {
    if width >= 64 { u64::MAX } else { (1u64 << width) - 1 }
}

fn bus_mask(busref: &BusRef) -> u64 {
    width_mask(busref.width) << busref.offset
}

fn read_bit(wire_state: &HashMap<usize, u64>, busref: &BusRef) -> bool {
    let val = wire_state.get(&wire_id(busref)).copied().unwrap_or(0);
    (val >> busref.offset) & 1 != 0
}

fn write_bit(wire_state: &mut HashMap<usize, u64>, busref: &BusRef, value: bool) {
    let bit = 1u64 << busref.offset;
    let entry = wire_state.entry(wire_id(busref)).or_insert(0);
    if value { *entry |= bit; } else { *entry &= !bit; }
}
