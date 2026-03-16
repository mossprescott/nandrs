mod wiring;
mod synth;
mod eval;

pub use synth::{synthesize, ChipWiring, RAMSpec, ROMSpec, MemorySystemSpec, SerialSpec, MemoryMap, RAMMap};
pub use eval::{initialize, ChipState, BusResident, RAMHandle, ROMHandle, SerialHandle};

/// Synthesize a chip and initialize its state in one step.
pub fn simulate<C>(chip: &crate::declare::IC<C>, memory_map: MemoryMap) -> ChipState
where
    C: Clone + crate::Reflect + Into<crate::component::Computational16>,
{
    initialize(synthesize(chip, memory_map))
}
