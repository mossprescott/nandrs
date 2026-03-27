use crate::component::{Computational, ComputationalCounts};
use crate::declare::BusRef;
use crate::nat::{IsGreater, N1, Nat};
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

// /// Nothing to expand.
// impl<Width: Nat> Component for Mux<Width> {
//     type Target = Mux<Width>;

//     fn expand(&self) -> Option<IC<Mux<Width>>> {
//         None
//     }
// }

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

// /// Nothing to expand.
// impl Component for Adder {
//     type Target = Adder;

//     fn expand(&self) -> Option<IC<Adder>> {
//         None
//     }
// }

/// The type of components that participate in computers for simulation purposes: this includes the
/// native components here in addition to the actual primitives of the Computational type.
#[derive(Clone, Reflect)]
pub enum Simulational<A: Nat, D: Nat> {
    Primitive(Computational<A, D>),
    Mux(Mux<D>),
    Mux1(Mux<N1>),
    Adder(Adder),
}

impl<A: Nat, D: Nat> From<Computational<A, D>> for Simulational<A, D> {
    fn from(c: Computational<A, D>) -> Self {
        Simulational::Primitive(c)
    }
}

impl<A: Nat, D: Nat> From<crate::component::Sequential<D>> for Simulational<A, D> {
    fn from(s: crate::component::Sequential<D>) -> Self {
        use crate::component::Sequential;
        Simulational::Primitive(match s {
            Sequential::Nand(n) => Computational::Nand(n),
            Sequential::Buffer(b) => Computational::Buffer(b),
            Sequential::Register(r) => Computational::Register(r),
        })
    }
}

impl<A: Nat, D: Nat> From<Mux<D>> for Simulational<A, D>
where
    D: IsGreater<N1>,
{
    fn from(c: Mux<D>) -> Self {
        Simulational::Mux(c)
    }
}

impl<A: Nat, D: Nat> From<Mux<N1>> for Simulational<A, D> {
    fn from(c: Mux<N1>) -> Self {
        Simulational::Mux1(c)
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
    pub mux1s: usize,
    pub adders: usize,
}

pub fn count_simulational<A: Nat, D: Nat>(components: &[Simulational<A, D>]) -> SimulationalCounts {
    let mut counts = SimulationalCounts {
        primitive: ComputationalCounts {
            nands: 0,
            buffers: 0,
            registers: 0,
            rams: 0,
            roms: 0,
            serials: 0,
            memory_systems: 0,
        },
        muxes: 0,
        mux1s: 0,
        adders: 0,
    };
    for comp in components {
        match comp {
            Simulational::Primitive(p) => match p {
                Computational::Nand(_) => counts.primitive.nands += 1,
                Computational::Buffer(_) => counts.primitive.buffers += 1,
                Computational::Register(_) => counts.primitive.registers += 1,
                Computational::RAM(_) => counts.primitive.rams += 1,
                Computational::ROM(_) => counts.primitive.roms += 1,
                Computational::Serial(_) => counts.primitive.serials += 1,
                Computational::MemorySystem(_) => counts.primitive.memory_systems += 1,
            },
            Simulational::Mux(_) => counts.muxes += 1,
            Simulational::Mux1(_) => counts.mux1s += 1,
            Simulational::Adder(_) => counts.adders += 1,
        }
    }
    counts
}
