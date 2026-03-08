use crate::declare::{IC, Interface, Reflect as _};
use crate::component::Sequential16;

/// Transform circuit description to a form for simulation.
pub fn synthesize(chip: &IC<Sequential16>) -> ChipState {
    // TODO

    ChipState {
        intf: chip.reflect(),
        name: chip.name().to_string(),

        // TODO
    }
}

/// Runtime state of a simulated chip, and access to its inputs and outputs.
///
/// Note: sequential chips will generally have both internal state (registers), and inputs and
/// outputs.
pub struct ChipState {
    pub intf: Interface,
    pub name: String,

    // TODO: store state of all wires and registers
}
impl ChipState {
    /// Set the value of an input for the next cycle.
    pub fn set(&mut self, name: &str, value: u64) {
        todo!()
    }

    /// Get the value of an output as of the last cycle.
    pub fn get(&self, name: &str) -> u64 {
        todo!()
    }

    /// Turn the crank: as it were, raise and lower the imaginary clock signal, causing
    /// stateful components (registers) to latch their inputs, then reevaluate.
    pub fn ticktock(&mut self) {
        todo!()
    }
}
