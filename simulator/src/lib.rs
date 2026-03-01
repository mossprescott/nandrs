#![allow(unused_variables, dead_code)]

pub mod declare;
pub use declare::*;


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