/// A simple evaluator for "combinational" chips, consisting only of Nands.
///
/// There is no clock and no state.

use std::collections::HashMap;
use std::rc::Rc;

use crate::component::{IC, Nand};
use crate::declare::{BusRef, Reflect};

/// Evaluate a chip given named input values, returning named output values.
///
/// Values are u64; for 1-bit signals use 0 or 1. For a multi-bit bus of width w,
/// bits 0..w-1 carry the value.
pub fn eval<'a, I>(chip: &IC<Nand>, inputs: I) -> HashMap<String, u64>
where
    I: IntoIterator<Item = (&'a str, u64)>,
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
            *entry = (*entry & !mask) | ((value << busref.offset) & mask);
        }
    }

    // Evaluate each Nand in order.
    for nand in &chip.components {
        let intf = nand.reflect();
        let a = read_bit(&wire_state, &intf.inputs["a"]);
        let b = read_bit(&wire_state, &intf.inputs["b"]);
        write_bit(&mut wire_state, &intf.outputs["out"], !(a & b));
    }

    // Read named outputs.
    intf.outputs
        .iter()
        .map(|(name, busref)| {
            let val = wire_state.get(&wire_id(busref)).copied().unwrap_or(0);
            (name.clone(), (val >> busref.offset) & width_mask(busref.width))
        })
        .collect()
}

fn wire_id(busref: &BusRef) -> usize {
    Rc::as_ptr(&busref.id) as usize
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
