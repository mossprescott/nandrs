mod wiring;
mod memory;
mod synth;
mod eval;

use crate::nat::Nat;
use crate::word::Storable;

pub use memory::{MemoryMap, RegionMap, RAMMap, ROMMap, SerialMap};
pub use synth::{synthesize, ChipWiring, RAMSpec, ROMSpec, MemorySystemSpec, SerialSpec};
pub use eval::{initialize, ChipState, BusResident, RAMHandle, ROMHandle, SerialHandle};

/// Synthesize a chip and initialize its state in one step.
pub fn simulate<C, A: Nat + Storable + Clone, D: Nat + Storable + Clone>(chip: &crate::declare::IC<C>, memory_map: MemoryMap) -> ChipState<A, D>
where
    C: Clone + crate::Reflect + Into<crate::component::Computational<A, D>>,
{
    initialize(synthesize(chip, memory_map))
}
