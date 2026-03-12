use std::collections::HashMap;

use crate::component::{Computational, Computational16};
use crate::declare::{IC, Reflect as _};

use super::wiring::{self, Indexes, WireID, WireIndex};

/// Static, synthesized description of the circuit's wiring. Computed once and never mutated.
pub struct ChipWiring {
    pub(super) component_wiring: Vec<wiring::ComponentWiring>,
    pub(super) input_wiring:  HashMap<String, wiring::WireRef>,
    pub(super) output_wiring: HashMap<String, wiring::WireRef>,
    pub(super) n_wires: usize,
    /// One entry per RAM component; the index is the device slot referenced by the wiring.
    pub ram_specs: Vec<RAMSpec>,
    /// One entry per ROM component; the index is the device slot referenced by the wiring.
    pub rom_specs: Vec<ROMSpec>,
    /// One entry per MemorySystem component, including the RAM region layout.
    pub ms_specs: Vec<MemorySystemSpec>,
}

/// Descriptor for a standalone RAM component.
pub struct RAMSpec { pub size: usize }

/// Descriptor for a ROM component.
pub struct ROMSpec { pub size: usize }

/// Descriptor for a MemorySystem component, including its RAM region layout.
pub struct MemorySystemSpec { pub regions: Vec<RAMMap> }

/// Descriptor for one contiguous RAM region in a memory map.
pub struct RAMMap {
    pub size: usize,
    pub base: usize,
}

/// Descriptor for the memory layout passed to [`synthesize`].
///
/// Specifies which regions exist and where they appear in the address space.
/// All actual data storage lives in device RAM instances created by [`super::initialize`].
pub struct MemoryMap {
    pub contents: Vec<RAMMap>,
}

impl MemoryMap {
    pub fn new(contents: Vec<RAMMap>) -> Self {
        MemoryMap { contents }
    }
}

/// Transform a circuit description into a pre-computed wiring layout.
///
/// No RAM or ROM buffers are allocated here. Call [`super::initialize`] to create a runnable
/// [`super::ChipState`].
///
/// Note: currently 16-bit words are assumed, but up to 64-bits wouldn't be a problem if the type
/// was generalized.
pub fn synthesize<C>(chip: &IC<C>, memory_map: MemoryMap) -> ChipWiring
where
    C: Clone + crate::Reflect + Into<Computational16>,
{
    let components: Vec<Computational16> = chip.components.iter().cloned().map(Into::into).collect();
    let mut memory_map = Some(memory_map);

    // Build wire_indexes: assign a contiguous WireIndex to every unique wire in the circuit.
    // This must be done before building component_wiring, which uses WireIndex directly.
    let mut wire_indexes: Indexes = HashMap::new();
    {
        let mut next_index = 0usize;
        let mut assign = |id: WireID| {
            if let std::collections::hash_map::Entry::Vacant(e) = wire_indexes.entry(id) {
                e.insert(WireIndex(next_index as u32));
                next_index += 1;
            }
        };
        let intf = chip.reflect();
        for b in intf.inputs.values()  { assign(WireID::from(b)); }
        for b in intf.outputs.values() { assign(WireID::from(b)); }
        for comp in &components {
            match comp {
                Computational::Nand(c) => {
                    let intf = c.reflect();
                    assign(WireID::from(&intf.inputs["a"]));
                    assign(WireID::from(&intf.inputs["b"]));
                    assign(WireID::from(&intf.outputs["out"]));
                }
                Computational::Register(c) => {
                    let intf = c.reflect();
                    assign(WireID::from(&intf.inputs["write"]));
                    assign(WireID::from(&intf.inputs["data_in"]));
                    assign(WireID::from(&intf.outputs["data_out"]));
                }
                Computational::RAM(c) => {
                    let intf = c.reflect();
                    assign(WireID::from(&intf.outputs["data_out"]));
                    assign(WireID::from(&intf.inputs["addr"]));
                    assign(WireID::from(&intf.inputs["write"]));
                    assign(WireID::from(&intf.inputs["data_in"]));
                }
                Computational::ROM(c) => {
                    let intf = c.reflect();
                    assign(WireID::from(&intf.outputs["out"]));
                    assign(WireID::from(&intf.inputs["addr"]));
                }
                Computational::MemorySystem(c) => {
                    let intf = c.reflect();
                    assign(WireID::from(&intf.outputs["data_out"]));
                    assign(WireID::from(&intf.inputs["addr"]));
                    assign(WireID::from(&intf.inputs["write"]));
                    assign(WireID::from(&intf.inputs["data_in"]));
                }
                Computational::Const(_) => {}
            }
        }
    }

    let mut ram_specs: Vec<RAMSpec> = Vec::new();
    let mut rom_specs: Vec<ROMSpec> = Vec::new();
    let mut ms_specs:  Vec<MemorySystemSpec>  = Vec::new();

    let component_wiring: Vec<wiring::ComponentWiring> = components.iter().map(|comp| {
        use wiring::ComponentWiring as CW;
        match comp {
            Computational::Nand(c)         => CW::Nand(wiring::NandWiring::new(c, &wire_indexes)),
            Computational::Register(c)     => CW::Register(wiring::RegisterWiring::new(c, &wire_indexes)),
            Computational::RAM(c)          => {
                let slot = ram_specs.len();
                ram_specs.push(RAMSpec { size: c.size });
                CW::RAM(wiring::RAMWiring::new(c, slot, &wire_indexes))
            }
            Computational::ROM(c)          => {
                let slot = rom_specs.len();
                rom_specs.push(ROMSpec { size: c.size });
                CW::ROM(wiring::ROMWiring::new(c, slot, &wire_indexes))
            }
            Computational::MemorySystem(c) => {
                let slot = ms_specs.len();
                let regions = memory_map.take().expect("only one MemorySystem supported").contents;
                ms_specs.push(MemorySystemSpec { regions });
                CW::MemorySystem(wiring::MemorySystemWiring::new(c, slot, &wire_indexes))
            }
            Computational::Const(_)        => CW::Const,
        }
    }).collect();

    let n_wires = wire_indexes.len();
    let intf = chip.reflect();
    ChipWiring {
        component_wiring,
        input_wiring:  intf.inputs.iter().map(|(name, b)|  (name.clone(), wiring::WireRef::new(b, &wire_indexes))).collect(),
        output_wiring: intf.outputs.iter().map(|(name, b)| (name.clone(), wiring::WireRef::new(b, &wire_indexes))).collect(),
        n_wires,
        ram_specs,
        rom_specs,
        ms_specs,
    }
}
