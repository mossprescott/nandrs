mod eval;
mod memory;
pub mod native;
mod synth;
mod wiring;

use crate::nat::Nat;
use crate::word::Storable;

pub use eval::{BusResident, ChipState, RAMHandle, ROMHandle, SerialHandle, initialize};
pub use memory::{MemoryMap, RAMMap, ROMMap, RegionMap, SerialMap};
pub use synth::{ChipWiring, MemorySystemSpec, OpCounts, RAMSpec, ROMSpec, SerialSpec, synthesize};

/// Synthesize a chip and initialize its state in one step.
pub fn simulate<C, A: Nat + Storable + Clone, D: Nat + Storable + Clone>(
    chip: &crate::declare::IC<C>,
    memory_map: MemoryMap,
) -> ChipState<A, D>
where
    C: Clone + crate::Reflect + Into<native::Simulational<A, D>>,
{
    initialize(synthesize(chip, memory_map))
}
