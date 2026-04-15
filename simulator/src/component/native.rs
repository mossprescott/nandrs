use std::collections::HashMap;

use crate::component::{Computational, ComputationalCounts};
use crate::declare::BusRef;
use crate::nat::Nat;
/// Components that aren't strictly primitive (or don't need to be), but which are provided as
/// "native" in that the simulator implements them directly for performance reasons.
use crate::{Chip, Input, Input1, Interface, Output, OutputBus, Reflect};

/// Mux: out = if sel { a1 } else { a0 }, applied bitwise across Width bits.
#[derive(Clone, Reflect, Chip)]
pub struct Mux<Width: Nat> {
    pub a0: Input<Width>,
    pub a1: Input<Width>,
    pub sel: Input1,
    pub out: OutputBus<Width>,
}

/// Runtime-width mux, produced by converting a `Mux<Width>`.  Used in `Simulational`.
#[derive(Clone)]
pub struct WiredMux {
    pub width: usize,
    pub a0: BusRef,
    pub a1: BusRef,
    pub sel: BusRef,
    pub out: BusRef,
}

impl Reflect for WiredMux {
    fn name(&self) -> String {
        "Mux".to_string()
    }

    fn reflect(&self) -> Interface {
        Interface {
            inputs: HashMap::from([
                ("a0".to_string(), self.a0),
                ("a1".to_string(), self.a1),
                ("sel".to_string(), self.sel),
            ]),
            outputs: HashMap::from([("out".to_string(), self.out)]),
        }
    }
}

impl<Width: Nat> From<Mux<Width>> for WiredMux {
    fn from(c: Mux<Width>) -> Self {
        WiredMux {
            width: Width::as_int(),
            a0: BusRef::from_input(c.a0),
            a1: BusRef::from_input(c.a1),
            sel: BusRef::from_input(c.sel),
            out: BusRef::from_output(c.out),
        }
    }
}

/// Single-bit slice off a multi-bit adder: adds three bits, producing a two-bit result.
///
/// Note that this primitive has the same interface and behavior as the project_02::FullAdder, which
/// reduces to only Nands. In fact, we require that to be the case, so that we can use one or the
/// other implementation for different purposes: this one, for efficient simulation; the other, for
/// counting the number of gates in a "realistic" implementation.
///
/// sum = 1s-digit of three-bit sum, carry = 2s-digit
#[derive(Clone, Reflect, Chip)]
pub struct Adder {
    /// "Left" input bit:
    pub a: Input1,
    /// "Right" input bit:
    pub b: Input1,
    /// "Carry-in" bit:
    pub c: Input1,

    /// 1s digit of a + b + c:
    pub sum: Output,
    /// 2s digit of a + b + c:
    pub carry: Output,
}

/// Arbitrary bit-width register, using a single word of state and a single operation to write.
pub struct Register<Width: Nat> {
    pub data_in: Input<Width>,
    pub write: Input1,
    pub data_out: OutputBus<Width>,
}

/// Runtime-width register, produced by converting a `Register<Width>`.  Used in `Simulational`.
#[derive(Clone)]
pub struct WiredRegister {
    pub width: usize,

    pub data_in: BusRef,
    pub write: BusRef,
    pub data_out: BusRef,
}

impl Reflect for WiredRegister {
    fn name(&self) -> String {
        "Register".to_string()
    }

    fn reflect(&self) -> Interface {
        Interface {
            inputs: HashMap::from([
                ("data_in".to_string(), self.data_in),
                ("write".to_string(), self.write),
            ]),
            outputs: HashMap::from([("data_out".to_string(), self.data_out)]),
        }
    }
}

impl<Width: Nat> From<Register<Width>> for WiredRegister {
    fn from(c: Register<Width>) -> Self {
        WiredRegister {
            width: Width::as_int(),
            data_in: BusRef::from_input(c.data_in),
            write: BusRef::from_input(c.write),
            data_out: BusRef::from_output(c.data_out),
        }
    }
}
/// The type of components that participate in computers for simulation purposes: this includes the
/// native components here in addition to the actual primitives of the Computational type.
#[derive(Clone, Reflect)]
pub enum Simulational<A: Nat, D: Nat> {
    Primitive(Computational<A, D>),
    Mux(WiredMux),
    Adder(Adder),
    Register(WiredRegister),
}

impl<A: Nat, D: Nat> From<Computational<A, D>> for Simulational<A, D> {
    fn from(c: Computational<A, D>) -> Self {
        Simulational::Primitive(c)
    }
}

impl<A: Nat, D: Nat> From<crate::component::Sequential> for Simulational<A, D> {
    fn from(s: crate::component::Sequential) -> Self {
        use crate::component::Sequential;
        Simulational::Primitive(match s {
            Sequential::Nand(n) => Computational::Nand(n),
            Sequential::Buffer(b) => Computational::Buffer(b),
            Sequential::DFF(r) => Computational::DFF(r),
        })
    }
}

impl<A: Nat, D: Nat> From<WiredMux> for Simulational<A, D> {
    fn from(c: WiredMux) -> Self {
        Simulational::Mux(c)
    }
}

impl<A: Nat, D: Nat, Width: Nat> From<Mux<Width>> for Simulational<A, D> {
    fn from(c: Mux<Width>) -> Self {
        Simulational::Mux(c.into())
    }
}

impl<A: Nat, D: Nat, Width: Nat> From<Register<Width>> for Simulational<A, D> {
    fn from(c: Register<Width>) -> Self {
        Simulational::Register(c.into())
    }
}

impl<A: Nat, D: Nat> From<Adder> for Simulational<A, D> {
    fn from(c: Adder) -> Self {
        Simulational::Adder(c)
    }
}

pub struct SimulationalCounts {
    pub primitive: ComputationalCounts,
    pub muxes: usize,
    pub adders: usize,
    pub registers: usize,
}

pub fn count_simulational<A: Nat, D: Nat>(components: &[Simulational<A, D>]) -> SimulationalCounts {
    let mut counts = SimulationalCounts {
        primitive: ComputationalCounts {
            nands: 0,
            buffers: 0,
            dffs: 0,
            rams: 0,
            roms: 0,
            serials: 0,
            memory_systems: 0,
        },
        muxes: 0,
        adders: 0,
        registers: 0,
    };
    for comp in components {
        match comp {
            Simulational::Primitive(p) => match p {
                Computational::Nand(_) => counts.primitive.nands += 1,
                Computational::Buffer(_) => counts.primitive.buffers += 1,
                Computational::DFF(_) => counts.primitive.dffs += 1,
                Computational::RAM(_) => counts.primitive.rams += 1,
                Computational::ROM(_) => counts.primitive.roms += 1,
                Computational::Serial(_) => counts.primitive.serials += 1,
                Computational::MemorySystem(_) => counts.primitive.memory_systems += 1,
            },
            Simulational::Mux(_) => counts.muxes += 1,
            Simulational::Adder(_) => counts.adders += 1,
            Simulational::Register(_) => counts.registers += 1,
        }
    }
    counts
}
