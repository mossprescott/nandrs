/// A simple evaluator for "combinational" chips, consisting only of Nands.
///
/// There is no clock and no state.

use std::collections::HashMap;
use std::rc::Rc;

use crate::component::Combinational;
use crate::declare::{BusRef, IC, Reflect};
use crate::nat::Nat;

/// Evaluate a chip given named input values, returning named output values.
///
/// Values are u64; for 1-bit signals use 0 or 1. For a multi-bit bus of width w,
/// bits 0..w-1 carry the value.
pub fn eval<'a, I, Width: Nat + Clone>(chip: &IC<Combinational<Width>>, inputs: I) -> HashMap<String, u64>
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
            Combinational::Const(c) => {
                let intf = c.reflect();
                let busref = &intf.outputs["out"];
                let id = wire_id(busref);
                let mask = bus_mask(busref);
                let entry = wire_state.entry(id).or_insert(0);
                *entry = (*entry & !mask) | ((c.value << busref.offset) & mask);
            }
            Combinational::Buffer(buffer) => {
                let intf = buffer.reflect();
                let a = read_bit(&wire_state, &intf.inputs["a"]);
                write_bit(&mut wire_state, &intf.outputs["out"], a);
            }
            Combinational::Mux(mux) => {
                let intf = mux.reflect();
                let sel = read_bit(&wire_state, &intf.inputs["sel"]);
                let src = if sel { &intf.inputs["a1"] } else { &intf.inputs["a0"] };
                let out = &intf.outputs["out"];
                let id = wire_id(out);
                let mask = bus_mask(out);
                let val = wire_state.get(&wire_id(src)).copied().unwrap_or(0);
                let shifted = ((val >> src.offset) & width_mask(out.width)) << out.offset;
                let entry = wire_state.entry(id).or_insert(0);
                *entry = (*entry & !mask) | shifted;
            }
            Combinational::Mux1(mux) => {
                let intf = mux.reflect();
                let sel = read_bit(&wire_state, &intf.inputs["sel"]);
                let src = if sel { &intf.inputs["a1"] } else { &intf.inputs["a0"] };
                let out = &intf.outputs["out"];
                let id = wire_id(out);
                let mask = bus_mask(out);
                let val = wire_state.get(&wire_id(src)).copied().unwrap_or(0);
                let shifted = ((val >> src.offset) & width_mask(out.width)) << out.offset;
                let entry = wire_state.entry(id).or_insert(0);
                *entry = (*entry & !mask) | shifted;
            }
            Combinational::Adder(adder) => {
                let intf = adder.reflect();
                let a = read_bit(&wire_state, &intf.inputs["a"]);
                let b = read_bit(&wire_state, &intf.inputs["b"]);
                let c = read_bit(&wire_state, &intf.inputs["c"]);
                let total = a as u64 + b as u64 + c as u64;
                let sum_ref = &intf.outputs["sum"];
                let carry_ref = &intf.outputs["carry"];
                write_bit(&mut wire_state, sum_ref, total & 1 != 0);
                write_bit(&mut wire_state, carry_ref, total & 2 != 0);
            }
        }
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
