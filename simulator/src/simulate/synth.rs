use std::collections::HashMap;
use std::fmt;

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

fn fmt_wire(w: wiring::WireIndex) -> impl fmt::Display { w.0 }

fn fmt_bit(b: wiring::BitRef) -> impl fmt::Display {
    struct D(wiring::BitRef);
    impl fmt::Display for D {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "w{}.{}", self.0.id.0, self.0.offset)
        }
    }
    D(b)
}

fn fmt_bus(b: wiring::WireRef) -> impl fmt::Display {
    struct D(wiring::WireRef);
    impl fmt::Display for D {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            if self.0.offset == 0 {
                write!(f, "w{}[{}]", self.0.id.0, self.0.width)
            } else {
                write!(f, "w{}[{}..{}]", self.0.id.0, self.0.offset, self.0.offset + self.0.width)
            }
        }
    }
    D(b)
}

impl fmt::Display for ChipWiring {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut nands = 0u32;
        let mut registers = 0u32;
        for comp in &self.component_wiring {
            match comp {
                wiring::ComponentWiring::Nand(_)     => nands += 1,
                wiring::ComponentWiring::Register(_) => registers += 1,
                _ => {}
            }
        }
        writeln!(f, "ChipWiring:")?;
        writeln!(f, "  wires:     {}", self.n_wires)?;
        writeln!(f, "  nands:     {}", nands)?;
        if registers > 0 { writeln!(f, "  registers: {}", registers)?; }
        for (i, s) in self.ram_specs.iter().enumerate() {
            writeln!(f, "  ram[{}]:    {} words", i, s.size)?;
        }
        for (i, s) in self.rom_specs.iter().enumerate() {
            writeln!(f, "  rom[{}]:    {} words", i, s.size)?;
        }
        for (i, ms) in self.ms_specs.iter().enumerate() {
            writeln!(f, "  memory[{}]:", i)?;
            for r in &ms.regions { writeln!(f, "    {} words @ 0x{:04x}", r.size, r.base)?; }
        }

        for (i, comp) in self.component_wiring.iter().enumerate() {
            match comp {
                wiring::ComponentWiring::Nand(n) =>
                    writeln!(f, "  [{i}] nand  a={} b={} out={}",
                        fmt_bit(n.a), fmt_bit(n.b), fmt_bit(n.out))?,
                wiring::ComponentWiring::ParallelNand(n) =>
                    writeln!(f, "  [{i}] pnand a={} b={} out={}",
                        fmt_bus(n.a), fmt_bus(n.b), fmt_bus(n.out))?,
                wiring::ComponentWiring::Register(r) =>
                    writeln!(f, "  [{i}] reg   write={} in={} out=w{}[16]",
                        fmt_bit(r.write), fmt_bus(r.data_in), r.data_out.0)?,
                wiring::ComponentWiring::ROM(r) =>
                    writeln!(f, "  [{i}] rom[{}]  addr={} out={}",
                        r.device_slot, fmt_bus(r.addr), fmt_bus(r.out))?,
                wiring::ComponentWiring::RAM(r) =>
                    writeln!(f, "  [{i}] ram[{}]  addr={} write={} in={} out={}",
                        r.device_slot, fmt_bus(r.addr), fmt_bit(r.write), fmt_bus(r.data_in), fmt_bus(r.out))?,
                wiring::ComponentWiring::MemorySystem(m) =>
                    writeln!(f, "  [{i}] mem[{}]  addr={} write={} in={} out={}",
                        m.device_slot, fmt_bus(m.addr), fmt_bit(m.write), fmt_bus(m.data_in), fmt_bus(m.out))?,
                wiring::ComponentWiring::Const =>
                    writeln!(f, "  [{i}] const")?,
            }
        }
        Ok(())
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
