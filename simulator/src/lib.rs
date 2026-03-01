#![allow(unused_variables, dead_code)]

use std::collections::HashMap;

/// Carries a single value which may be interpreted as on/off, true/false, 1/0, etc. Logically
/// equivalent to a one-bit-wide Bus.
pub struct Wire {}

/// A (logical) collection of wires, carrying related signals, as in the bits of a value treated as binary data.
/// Typical width is the "word size" of the simulated machine, or less.
pub struct Bus {
    bits: usize
}

#[derive(Debug, PartialEq)]
pub enum ConnectionWidth {
    /// A connection carrying a single signal.
    Wire,

    /// A connection carrying multiple parallel signals, typically at least 2 and most often the
    /// word size of the simulated machine.
    Bus { width: u32 },
}

/// Declare the connections for a certain type of component. When an instance is added to an
/// Assembly, all connected inputs and outputs must match for width.
/// Any unconnected input may be assumed to be zero; any unconnected output is
pub trait Component {
    fn inputs(&self) -> HashMap<String, ConnectionWidth>;
    fn outputs(&self) -> HashMap<String, ConnectionWidth>;
}

// Some commonly-used connection names:

// const IN: String = String::from("in");
// const A: String = String::from("a");
// const B: String = String::from("b");
// const OUT: String = String::from("out");


/// A collection of components which forms a unit of circuit construction. Provides output signals
/// (wires/buses), and accepts signals from elsewhere as inputs.
pub struct Assembly {
    // TODO
}

// pub trait Component {
//     fn build(&self) -> Assembly;
// }

/// *The* primitive gate. All other circuits are built from this.
// pub struct Nand {
//     a: Wire,
//     b: Wire,
// }
// impl Component for Nand {
//     pub fn build(&self) -> Assembly {
//         // TODO: trivially-compose a single Nand for use with other components.
//         Assembly {}
//     }
// }

/// An assembly with no inputs connected yet; for creating cyclical references.
pub fn lazy() -> Assembly {
    // TODO
    Assembly {}
}


/// Encapsulates a chip-design, along with the current state of all of its components during
/// simulation.
pub struct Chip {
    // TODO
}

impl Chip {
    pub fn set(&mut self, name: &str, value: bool) {
        // TODO
    }

    pub fn get(&self, name: &str) -> bool {
        // TODO
        false
    }
}


// TODO: ClockedChip, providing clock signal and tick/tock operations
// TODO: Computer, providing standard I/O signals also?


/// Given a description of all connections, compile the assembly to its ready-to-simulate form.
pub fn build(chip: Assembly) -> Chip {
    // TODO
    Chip {}
}