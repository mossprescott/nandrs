use std::collections::HashMap;
use std::rc::Rc;

use crate::declare::{BusRef, IC, Interface, Reflect as _};
use crate::component::{Sequential, Sequential16};

/// Transform circuit description to a form for simulation.
pub fn synthesize(chip: &IC<Sequential16>) -> ChipState {
    let mut reg_state: HashMap<usize, u64> = HashMap::new();
    for comp in &chip.components {
        if let Sequential::Register(reg) = comp {
            let intf = reg.reflect();
            reg_state.insert(wire_id(&intf.outputs["out"]), 0);
        }
    }
    let mut state = ChipState {
        intf: chip.reflect(),
        name: chip.name().to_string(),
        components: chip.components.clone(),
        input_vals: HashMap::new(),
        wire_state: HashMap::new(),
        reg_state,
    };
    state.evaluate();
    state
}

/// Runtime state of a simulated chip, and access to its inputs and outputs.
pub struct ChipState {
    intf: Interface,
    name: String,
    components: Vec<Sequential16>,
    input_vals: HashMap<String, u64>,
    wire_state: HashMap<usize, u64>,
    reg_state: HashMap<usize, u64>,
}

impl ChipState {
    /// Set the value of an input for the next cycle.
    pub fn set(&mut self, name: &str, value: u64) {
        self.input_vals.insert(name.to_string(), value);
    }

    /// Get the value of an output as of the last cycle.
    pub fn get(&self, name: &str) -> u64 {
        self.intf.outputs.get(name)
            .map(|b| read_bus(&self.wire_state, b))
            .unwrap_or(0)
    }

    /// Turn the crank: latch registers then re-evaluate combinational logic.
    pub fn ticktock(&mut self) {
        // Evaluate with current inputs so wire_state reflects this cycle.
        self.evaluate();

        // Latch registers.
        for comp in &self.components {
            if let Sequential::Register(reg) = comp {
                let intf = reg.reflect();
                if read_bit(&self.wire_state, &intf.inputs["load"]) {
                    let val = read_bus(&self.wire_state, &intf.inputs["data"]);
                    self.reg_state.insert(wire_id(&intf.outputs["out"]), val);
                }
            }
        }

        // Re-evaluate so outputs reflect the new register state.
        self.evaluate();
    }

    fn evaluate(&mut self) {
        let mut ws: HashMap<usize, u64> = HashMap::new();

        // Seed chip inputs.
        for (name, &val) in &self.input_vals {
            if let Some(b) = self.intf.inputs.get(name) {
                write_bus(&mut ws, b, val);
            }
        }

        // Seed register outputs.
        for (&id, &val) in &self.reg_state {
            ws.insert(id, val);
        }

        // Evaluate Nands in order.
        for comp in &self.components {
            if let Sequential::Nand(nand) = comp {
                let intf = nand.reflect();
                let a = read_bit(&ws, &intf.inputs["a"]);
                let b = read_bit(&ws, &intf.inputs["b"]);
                write_bit(&mut ws, &intf.outputs["out"], !(a & b));
            }
        }

        self.wire_state = ws;
    }
}

fn wire_id(busref: &BusRef) -> usize {
    Rc::as_ptr(&busref.id) as usize
}

fn width_mask(width: usize) -> u64 {
    if width >= 64 { u64::MAX } else { (1u64 << width) - 1 }
}

fn read_bus(ws: &HashMap<usize, u64>, b: &BusRef) -> u64 {
    let raw = ws.get(&wire_id(b)).copied().unwrap_or(0);
    (raw >> b.offset) & width_mask(b.width)
}

fn write_bus(ws: &mut HashMap<usize, u64>, b: &BusRef, value: u64) {
    let mask = width_mask(b.width);
    let entry = ws.entry(wire_id(b)).or_insert(0);
    *entry = (*entry & !(mask << b.offset)) | ((value & mask) << b.offset);
}

fn read_bit(ws: &HashMap<usize, u64>, b: &BusRef) -> bool {
    (ws.get(&wire_id(b)).copied().unwrap_or(0) >> b.offset) & 1 != 0
}

fn write_bit(ws: &mut HashMap<usize, u64>, b: &BusRef, value: bool) {
    let entry = ws.entry(wire_id(b)).or_insert(0);
    let bit = 1u64 << b.offset;
    if value { *entry |= bit; } else { *entry &= !bit; }
}
