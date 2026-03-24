use crate::declare::BusRef;
use crate::nat::{N16, Nat};
use crate::{Chip, Component, IC, Input, Input1, Interface, Output, OutputBus, Reflect};

// - Nand (Combinational)

/// The single primitive: true if either input is false.
#[derive(Clone, Reflect, Chip)]
pub struct Nand {
    pub a: Input1,
    pub b: Input1,
    pub out: Output,
}

/// "Gate" that just connects its input to its output without modifying it. For our purposes,
/// this is useful for connecting an input directly to an output in an IC.
#[derive(Clone, Reflect)]
pub struct Buffer {
    pub a: Input1,
    pub out: Output,
}

/// Nothing to expand; Buffer is primitive.
impl Component for Buffer {
    type Target = Buffer;

    fn expand(&self) -> Option<IC<Buffer>> {
        None
    }
}

/// Type of components that participate in "combinational" circuits:
/// - most importantly `Nand`
/// - `Buffer` for pass-through connections
pub enum Combinational {
    Nand(Nand),
    Buffer(Buffer),
}

impl From<Nand> for Combinational {
    fn from(c: Nand) -> Self {
        Combinational::Nand(c)
    }
}
impl From<Buffer> for Combinational {
    fn from(c: Buffer) -> Self {
        Combinational::Buffer(c)
    }
}

impl Reflect for Combinational {
    fn reflect(&self) -> Interface {
        match self {
            Self::Nand(c) => c.reflect(),
            Self::Buffer(c) => c.reflect(),
        }
    }
    fn name(&self) -> String {
        match self {
            Self::Nand(c) => c.name(),
            Self::Buffer(c) => c.name(),
        }
    }
}

pub struct CombinationalCounts {
    pub nands: usize,
    pub buffers: usize,
}

pub fn count_combinational(components: &[Combinational]) -> CombinationalCounts {
    let mut counts = CombinationalCounts {
        nands: 0,
        buffers: 0,
    };
    for comp in components {
        match comp {
            Combinational::Nand(_) => counts.nands += 1,
            Combinational::Buffer(_) => counts.buffers += 1,
        }
    }
    counts
}

// - Registers (Sequential)

#[derive(Clone, Reflect, Chip)]
pub struct Register<Width: Nat> {
    pub data_in: Input<Width>,
    pub write: Input1,
    pub data_out: OutputBus<Width>,
}

/// Nothing to expand; Register is primitive for the simulator we envisage.
impl<Width: Nat> Component for Register<Width> {
    type Target = Register<Width>;

    fn expand(&self) -> Option<IC<Register<Width>>> {
        None
    }
}

pub type Register16 = Register<N16>;

/// Type of components that participate in "sequential" circuits of a defined width: Combinational
/// and `Register<Width>`.
#[derive(Clone)]
pub enum Sequential<Width: Nat> {
    Nand(Nand),
    Buffer(Buffer),
    Register(Register<Width>),
}

impl<Width: Nat> From<Nand> for Sequential<Width> {
    fn from(c: Nand) -> Self {
        Sequential::Nand(c)
    }
}
impl<Width: Nat> From<Buffer> for Sequential<Width> {
    fn from(c: Buffer) -> Self {
        Sequential::Buffer(c)
    }
}
impl<Width: Nat> From<Register<Width>> for Sequential<Width> {
    fn from(c: Register<Width>) -> Self {
        Sequential::Register(c)
    }
}

impl<Width: Nat + Clone> Reflect for Sequential<Width> {
    fn reflect(&self) -> Interface {
        match self {
            Self::Nand(c) => c.reflect(),
            Self::Buffer(c) => c.reflect(),
            Self::Register(c) => c.reflect(),
        }
    }
    fn name(&self) -> String {
        match self {
            Self::Nand(c) => c.name(),
            Self::Buffer(c) => c.name(),
            Self::Register(c) => c.name(),
        }
    }
}

impl<Width: Nat> Component for Sequential<Width> {
    type Target = Self;

    fn expand(&self) -> Option<IC<Self::Target>> {
        None
    }
}

pub type Sequential16 = Sequential<N16>;

pub struct SequentialCounts {
    pub nands: usize,
    pub buffers: usize,
    pub registers: usize,
}

pub fn count_sequential<W: Nat>(components: &[Sequential<W>]) -> SequentialCounts {
    let mut counts = SequentialCounts {
        nands: 0,
        buffers: 0,
        registers: 0,
    };
    for comp in components {
        match comp {
            Sequential::Nand(_) => counts.nands += 1,
            Sequential::Buffer(_) => counts.buffers += 1,
            Sequential::Register(_) => counts.registers += 1,
        }
    }
    counts
}

// - Memory and I/O (Computational)

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

/// Nothing to expand; RAM is primitive for the simulator.
impl<A: Nat, D: Nat> Component for RAM<A, D> {
    type Target = RAM<A, D>;

    fn expand(&self) -> Option<IC<RAM<A, D>>> {
        None
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

/// Nothing to expand; ROM is primitive for the simulator.
impl<A: Nat, D: Nat> Component for ROM<A, D> {
    type Target = ROM<A, D>;

    fn expand(&self) -> Option<IC<ROM<A, D>>> {
        None
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

impl<W: Nat> Component for Serial<W> {
    type Target = Serial<W>;
    fn expand(&self) -> Option<IC<Serial<W>>> {
        None
    }
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

/// Nothing to expand; MemorySystem is primitive for the simulator.
impl<A: Nat, D: Nat> Component for MemorySystem<A, D> {
    type Target = MemorySystem<A, D>;

    fn expand(&self) -> Option<IC<MemorySystem<A, D>>> {
        None
    }
}

// - Computational

/// Type of components that participate in computers, including logic, registers, memory, and I/O.
#[derive(Clone)]
pub enum Computational<A: Nat, D: Nat> {
    // combinational:
    Nand(Nand),
    Buffer(Buffer),
    // sequential:
    Register(Register<D>),
    // computational:
    RAM(RAM<A, D>),
    ROM(ROM<A, D>),
    Serial(Serial<D>),
    MemorySystem(MemorySystem<A, D>),
}

impl<A: Nat + Clone, D: Nat + Clone> Reflect for Computational<A, D> {
    fn reflect(&self) -> Interface {
        match self {
            Self::Nand(c) => c.reflect(),
            Self::Buffer(c) => c.reflect(),
            Self::Register(c) => c.reflect(),
            Self::RAM(c) => c.reflect(),
            Self::ROM(c) => c.reflect(),
            Self::Serial(c) => c.reflect(),
            Self::MemorySystem(c) => c.reflect(),
        }
    }
    fn name(&self) -> String {
        match self {
            Self::Nand(c) => c.name(),
            Self::Buffer(c) => c.name(),
            Self::Register(c) => c.name(),
            Self::RAM(c) => c.name(),
            Self::ROM(c) => c.name(),
            Self::Serial(c) => c.name(),
            Self::MemorySystem(c) => c.name(),
        }
    }
}

impl<A: Nat, D: Nat> Component for Computational<A, D> {
    type Target = Self;

    fn expand(&self) -> Option<IC<Self::Target>> {
        None
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
    pub registers: usize,
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
        registers: 0,
        rams: 0,
        roms: 0,
        serials: 0,
        memory_systems: 0,
    };
    for comp in components {
        match comp {
            Computational::Nand(_) => counts.nands += 1,
            Computational::Buffer(_) => counts.buffers += 1,
            Computational::Register(_) => counts.registers += 1,
            Computational::RAM(_) => counts.rams += 1,
            Computational::ROM(_) => counts.roms += 1,
            Computational::Serial(_) => counts.serials += 1,
            Computational::MemorySystem(_) => counts.memory_systems += 1,
        }
    }
    counts
}

impl<A: Nat, D: Nat> From<Combinational> for Computational<A, D> {
    fn from(c: Combinational) -> Self {
        match c {
            Combinational::Nand(n) => Computational::Nand(n),
            Combinational::Buffer(b) => Computational::Buffer(b),
        }
    }
}

impl<A: Nat, D: Nat> From<Sequential<D>> for Computational<A, D> {
    fn from(s: Sequential<D>) -> Self {
        match s {
            Sequential::Nand(n) => Computational::Nand(n),
            Sequential::Buffer(n) => Computational::Buffer(n),
            Sequential::Register(r) => Computational::Register(r),
        }
    }
}
