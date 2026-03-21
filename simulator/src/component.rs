use crate::{Component, IC, Input, InputBus, Output, OutputBus, Reflect, Chip, Interface};
use crate::declare::BusRef;
use crate::nat::{Nat, N1, N16, IsGreater};

// - Nand (Combinational)

/// The single primitive: true if either input is false.
#[derive(Clone, Reflect, Chip)]
pub struct Nand {
    pub a: Input,
    pub b: Input,
    pub out: Output,
}

/// "Gate" that just connects its input to its output without modifying it. For our purposes,
/// this is useful for connecting an input directly to an ouput in an IC.
#[derive(Clone, Reflect)]
pub struct Buffer {
    pub a: Input,
    pub out: Output,
}

/// Nothing to expand; Buffer is primitive.
impl Component for Buffer {
    type Target = Buffer;

    fn expand(&self) -> Option<IC<Buffer>> {
        None
    }
}

/// The Mux primitive: out = if sel { a1 } else { a0 }, applied bitwise across Width bits.
#[derive(Clone, Reflect, Chip)]
pub struct Mux<Width: Nat> {
    pub a0: InputBus<Width>,
    pub a1: InputBus<Width>,
    pub sel: Input,
    pub out: OutputBus<Width>,
}

/// Nothing to expand; Mux is primitive.
impl<Width: Nat> Component for Mux<Width> {
    type Target = Mux<Width>;

    fn expand(&self) -> Option<IC<Mux<Width>>> {
        None
    }
}

pub type Mux1 = Mux<N1>;
pub type Mux16 = Mux<N16>;

/// Single-bit slice off a multi-bit adder: adds three bits, producing a two-bit result
///
/// sum = 1s-digit of three-bit sum, carry = 2s-digit
#[derive(Clone, Reflect, Chip)]
pub struct FullAdder {
    /// "Left" input bit:
    pub a: Input,
    /// "Right" input bit:
    pub b: Input,
    /// "Carry-in" bit:
    pub c: Input,

    /// 1s digit of a + b + c:
    pub sum: Output,
    /// 2s digit of a + b + c:
    pub carry: Output,
}

/// Nothing to expand; FullAdder is primitive.
impl Component for FullAdder {
    type Target = FullAdder;

    fn expand(&self) -> Option<IC<FullAdder>> { None }
}

/// Type of components that participate in "combinational" circuits:
/// - most importantly Nand
/// - Buffer for pass-through connections
/// - Mux, included because it makes simulation significantly more efficient
pub enum Combinational<Width: Nat> {
    Nand(Nand),
    Buffer(Buffer),
    Mux(Mux<Width>),
    /// For conditionalizing chains of logic, we need a single-bit Mux as well.
    Mux1(Mux1),
    Adder(FullAdder),
}

impl<Width: Nat> From<Nand>  for Combinational<Width> { fn from(c: Nand)  -> Self { Combinational::Nand(c)  } }
impl<Width: Nat> From<Buffer> for Combinational<Width> { fn from(c: Buffer) -> Self { Combinational::Buffer(c) } }
impl<Width: Nat> From<Mux<Width>> for Combinational<Width>
  where Width: IsGreater<N1>
{
    fn from(c: Mux<Width>) -> Self { Combinational::Mux(c) }
}
impl<Width: Nat> From<Mux<N1>> for Combinational<Width> { fn from(c: Mux<N1>) -> Self { Combinational::Mux1(c) } }
impl<Width: Nat> From<FullAdder> for Combinational<Width> { fn from(c: FullAdder) -> Self { Combinational::Adder(c) } }

impl<Width: Nat + Clone> Reflect for Combinational<Width> {
    fn reflect(&self) -> Interface {
        match self {
            Self::Nand(c)   => c.reflect(),
            Self::Buffer(c) => c.reflect(),
            Self::Mux(c)    => c.reflect(),
            Self::Mux1(c)   => c.reflect(),
            Self::Adder(c)  => c.reflect(),
        }
    }
    fn name(&self) -> String {
        match self {
            Self::Nand(c)   => c.name(),
            Self::Buffer(c) => c.name(),
            Self::Mux(c)    => c.name(),
            Self::Mux1(c)   => c.name(),
            Self::Adder(c)  => c.name(),
        }
    }
}

// - Registers (Sequential)

#[derive(Clone, Reflect, Chip)]
pub struct Register<Width: Nat> {
    pub data_in: InputBus<Width>,
    pub write: Input,
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
    Mux(Mux<Width>),
    Mux1(Mux1),
    Adder(FullAdder),
    Register(Register<Width>),
}

impl<Width: Nat> From<Nand>            for Sequential<Width> { fn from(c: Nand)            -> Self { Sequential::Nand(c)     } }
impl<Width: Nat> From<Buffer>          for Sequential<Width> { fn from(c: Buffer)          -> Self { Sequential::Buffer(c)   } }
impl<Width: Nat> From<Mux<Width>>      for Sequential<Width>
  where Width: IsGreater<N1>
{
    fn from(c: Mux<Width>) -> Self { Sequential::Mux(c) }
}
impl<Width: Nat> From<Mux<N1>>        for Sequential<Width> { fn from(c: Mux<N1>)        -> Self { Sequential::Mux1(c)    } }
impl<Width: Nat> From<FullAdder>       for Sequential<Width> { fn from(c: FullAdder)       -> Self { Sequential::Adder(c)   } }
impl<Width: Nat> From<Register<Width>> for Sequential<Width> { fn from(c: Register<Width>) -> Self { Sequential::Register(c) } }

impl<Width: Nat + Clone> Reflect for Sequential<Width> {
    fn reflect(&self) -> Interface {
        match self {
            Self::Nand(c)     => c.reflect(),
            Self::Buffer(c)   => c.reflect(),
            Self::Mux(c)      => c.reflect(),
            Self::Mux1(c)     => c.reflect(),
            Self::Adder(c)    => c.reflect(),
            Self::Register(c) => c.reflect(),
        }
    }
    fn name(&self) -> String {
        match self {
            Self::Nand(c)     => c.name(),
            Self::Buffer(c)   => c.name(),
            Self::Mux(c)      => c.name(),
            Self::Mux1(c)     => c.name(),
            Self::Adder(c)    => c.name(),
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


// - Memory and I/O (Computational)

/// Simple, writable memory. The simulator supplies an implmentation when it finds one of these.
#[derive(Clone, Reflect)]
pub struct RAM<A: Nat, D: Nat> {
    /// Capacity of the RAM in words; <= 2^address_bits. Valid addresses are 0 to size-1.
    pub size: usize,

    pub addr: InputBus<A>,

    pub write: Input,
    pub data_in: InputBus<D>,

    pub data_out: OutputBus<D>,
}

// Note: this is not the Chip trait, du to the extra arg.
impl<A: Nat, D: Nat> RAM<A, D> {
    pub fn chip(size: usize) -> Self {
        RAM { size, addr: InputBus::new(), write: Input::new(), data_in: InputBus::new(), data_out: OutputBus::<D>::new() }
    }
}

/// Nothing to expand; RAM is primitive for the simulator.
impl<A: Nat, D: Nat> Component for RAM<A, D> {
    type Target = RAM<A, D>;

    fn expand(&self) -> Option<IC<RAM<A, D>>> {
        None
    }
}

/// Simple, read-only memory. The simulator supplies an implmentation when it finds one of these.
#[derive(Clone, Reflect)]
pub struct ROM<A: Nat, D: Nat> {
    /// Capacity of the ROM in words; <= 2^address_bits. Valid addresses are 0 to size-1.
    pub size: usize,

    pub addr: InputBus<A>,

    pub out: OutputBus<D>,
}

// Note: this is not the Chip trait, du to the extra arg.
impl<A: Nat, D: Nat> ROM<A, D> {
    pub fn chip(size: usize) -> Self {
        ROM { size: size, addr: InputBus::<A>::new(), out: OutputBus::<D>::new() }
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
    pub data_in: InputBus<Width>,

    /// Write strobe: when 1, data_in is latched to the external device.
    pub write: Input,
}

impl<W: Nat> Component for Serial<W> {
    type Target = Serial<W>;
    fn expand(&self) -> Option<IC<Serial<W>>> { None }
}

/// Abstracted writable memory system; presents the same interface as a RAM, but the simulator
/// allows an arbitrary implementation to be supplied. This is analogous to dropping a CPU into
/// a new system where some other chip is in charge of managing the bus.
#[derive(Clone, Reflect, Chip)]
pub struct MemorySystem<A: Nat, D: Nat> {
    pub addr: InputBus<A>,

    pub write: Input,
    pub data_in: InputBus<D>,

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
    Mux(Mux<D>),
    Mux1(Mux1),
    Adder(FullAdder),
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
            // combinational:
            Self::Nand(c)         => c.reflect(),
            Self::Buffer(c)       => c.reflect(),
            Self::Mux(c)          => c.reflect(),
            Self::Mux1(c)         => c.reflect(),
            Self::Adder(c)        => c.reflect(),
            // sequential:
            Self::Register(c)     => c.reflect(),
            // computational:
            Self::RAM(c)          => c.reflect(),
            Self::ROM(c)          => c.reflect(),
            Self::Serial(c)       => c.reflect(),
            Self::MemorySystem(c) => c.reflect(),
        }
    }
    fn name(&self) -> String {
        match self {
            // combinational:
            Self::Nand(c)         => c.name(),
            Self::Buffer(c)       => c.name(),
            Self::Mux(c)          => c.name(),
            Self::Mux1(c)         => c.name(),
            Self::Adder(c)        => c.name(),
            // sequential:
            Self::Register(c)     => c.name(),
            // computational:
            Self::RAM(c)          => c.name(),
            Self::ROM(c)          => c.name(),
            Self::Serial(c)       => c.name(),
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

pub type RAM16           = RAM<N16, N16>;
pub type ROM16           = ROM<N16, N16>;
pub type Serial16        = Serial<N16>;
pub type MemorySystem16  = MemorySystem<N16, N16>;
pub type Computational16 = Computational<N16, N16>;

impl<A: Nat, D: Nat> From<Sequential<D>> for Computational<A, D> {
    fn from(s: Sequential<D>) -> Self {
        match s {
            Sequential::Nand(n)     => Computational::Nand(n),
            Sequential::Buffer(n)   => Computational::Buffer(n),
            Sequential::Mux(m)      => Computational::Mux(m),
            Sequential::Mux1(m)     => Computational::Mux1(m),
            Sequential::Adder(m)    => Computational::Adder(m),
            Sequential::Register(r) => Computational::Register(r),
        }
    }
}
