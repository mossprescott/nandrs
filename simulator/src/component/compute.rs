//! Computational primitives: `RAM`, `ROM`, `Serial`, and `MemorySystem`, plus the `Computational`
//! enum combining them with all sequential and combinational components.

use crate::declare::BusRef;
use crate::nat::{N16, Nat};
use crate::{Chip, Input, Input1, Interface, OutputBus, Reflect};

use super::{Buffer, Combinational, DFF, Nand, Sequential};

/// Simple, writable memory. The simulator supplies an implementation when it finds one of these.
#[derive(Clone, Reflect)]
pub struct RAM<A: Nat, D: Nat> {
    /// Capacity of the RAM in words; <= 2^address_bits. Valid addresses are 0 to size-1.
    pub size: usize,

    pub addr: Input<A>,

    pub write: Input1,
    pub data_in: Input<D>,

    pub data_out: OutputBus<D>,
}

// Note: this is not the Chip trait, due to the extra arg.
impl<A: Nat, D: Nat> RAM<A, D> {
    pub fn chip(size: usize) -> Self {
        RAM {
            size,
            addr: Input::new(),
            write: Input::new(),
            data_in: Input::new(),
            data_out: OutputBus::<D>::new(),
        }
    }
}

/// Simple, read-only memory. The simulator supplies an implementation when it finds one of these.
#[derive(Clone, Reflect)]
pub struct ROM<A: Nat, D: Nat> {
    /// Capacity of the ROM in words; <= 2^address_bits. Valid addresses are 0 to size-1.
    pub size: usize,

    pub addr: Input<A>,

    pub out: OutputBus<D>,
}

// Note: this is not the Chip trait, due to the extra arg.
impl<A: Nat, D: Nat> ROM<A, D> {
    pub fn chip(size: usize) -> Self {
        ROM {
            size: size,
            addr: Input::<A>::new(),
            out: OutputBus::<D>::new(),
        }
    }
}

/// Read/write one word at a time from/to the outside world. Could represent a directly-connected
/// keyboard (as in the original design), or a serial port, a debug interface, or some combination
/// of the above.
///
/// The chip sees data_out (read from the device) and can write via data_in + write.
/// The simulator provides the backing store; the harness can push/pull values through a handle.
#[derive(Clone, Reflect, Chip)]
pub struct Serial<Width: Nat> {
    /// Data output: value made available to the chip by the external device.
    pub data_out: OutputBus<Width>,

    /// Data input: value the chip wants to send to the external device.
    pub data_in: Input<Width>,

    /// Write strobe: when 1, data_in is latched to the external device.
    pub write: Input1,
}

/// Abstracted writable memory system; presents the same interface as a RAM, but the simulator
/// allows an arbitrary implementation to be supplied. This is analogous to dropping a CPU into
/// a new system where some other chip is in charge of managing the bus.
#[derive(Clone, Reflect, Chip)]
pub struct MemorySystem<A: Nat, D: Nat> {
    pub addr: Input<A>,

    pub write: Input1,
    pub data_in: Input<D>,

    pub data_out: OutputBus<D>,
}

/// Type of components that participate in computers, including logic, registers, memory, and I/O.
#[derive(Clone, Reflect)]
pub enum Computational<A: Nat, D: Nat> {
    Nand(Nand),
    Buffer(Buffer),
    DFF(DFF),
    RAM(RAM<A, D>),
    ROM(ROM<A, D>),
    Serial(Serial<D>),
    MemorySystem(MemorySystem<A, D>),
}

impl<A: Nat, D: Nat> From<Combinational> for Computational<A, D> {
    fn from(c: Combinational) -> Self {
        match c {
            Combinational::Nand(n) => Computational::Nand(n),
            Combinational::Buffer(b) => Computational::Buffer(b),
        }
    }
}

impl<A: Nat, D: Nat> From<Sequential> for Computational<A, D> {
    fn from(s: Sequential) -> Self {
        match s {
            Sequential::Nand(n) => Computational::Nand(n),
            Sequential::Buffer(n) => Computational::Buffer(n),
            Sequential::DFF(r) => Computational::DFF(r),
        }
    }
}

pub type RAM16 = RAM<N16, N16>;
pub type ROM16 = ROM<N16, N16>;
pub type Serial16 = Serial<N16>;
pub type MemorySystem16 = MemorySystem<N16, N16>;
pub type Computational16 = Computational<N16, N16>;

pub struct ComputationalCounts {
    pub nands: usize,
    pub buffers: usize,
    pub dffs: usize,
    pub rams: usize,
    pub roms: usize,
    pub serials: usize,
    pub memory_systems: usize,
}

pub fn count_computational<A: Nat, D: Nat>(
    components: &[Computational<A, D>],
) -> ComputationalCounts {
    let mut counts = ComputationalCounts {
        nands: 0,
        buffers: 0,
        dffs: 0,
        rams: 0,
        roms: 0,
        serials: 0,
        memory_systems: 0,
    };
    for comp in components {
        match comp {
            Computational::Nand(_) => counts.nands += 1,
            Computational::Buffer(_) => counts.buffers += 1,
            Computational::DFF(_) => counts.dffs += 1,
            Computational::RAM(_) => counts.rams += 1,
            Computational::ROM(_) => counts.roms += 1,
            Computational::Serial(_) => counts.serials += 1,
            Computational::MemorySystem(_) => counts.memory_systems += 1,
        }
    }
    counts
}
