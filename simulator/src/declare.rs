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
