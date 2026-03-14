use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::fmt;

use crate::component::{Computational, Computational16};
use crate::declare::{BusRef, IC, Reflect as _};

use super::wiring::{self, Indexes, WireID, WireIndex, WireRef};

/// Static, synthesized description of the circuit's wiring. Computed once and never mutated.
pub struct ChipWiring {
    pub(super) component_wiring: Vec<wiring::ComponentWiring>,
    pub(super) input_wiring:  HashMap<String, wiring::WireRef>,
    pub(super) output_wiring: HashMap<String, wiring::WireRef>,
    pub(super) const_wiring: Vec<wiring::ConstWiring>,
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

fn fmt_bit(b: wiring::BitRef) -> impl fmt::Display {
    struct D(wiring::BitRef);
    impl fmt::Display for D {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "w{}[{}]", self.0.id.0, self.0.offset)
        }
    }
    D(b)
}

fn fmt_wire(wr: wiring::WireRef) -> impl fmt::Display {
    struct D(wiring::WireRef);
    impl fmt::Display for D {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            if self.0.width == 1 {
                write!(f, "w{}[{}]", self.0.id.0, self.0.offset)
            }
            else {
                write!(f, "w{}[{}..{}]", self.0.id.0, self.0.offset, self.0.offset + self.0.width)
            }
        }
    }
    D(wr)
}

fn fmt_component(comp: &wiring::ComponentWiring) -> impl fmt::Display + '_ {
    struct D<'a>(&'a wiring::ComponentWiring);
    impl fmt::Display for D<'_> {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self.0 {
                wiring::ComponentWiring::Nand(n) =>
                    write!(f, "nand a={} b={} out={}", fmt_bit(n.a), fmt_bit(n.b), fmt_bit(n.out)),
                wiring::ComponentWiring::Mux(m) =>
                    write!(f, "mux sel={} a0=w{}[..] a1=w{}[..] out=w{}[..]",
                        fmt_bit(m.sel), m.a0.0, m.a1.0, m.out.0),
                wiring::ComponentWiring::Register(r) =>
                    write!(f, "reg write={} in=w{}[..] out=w{}[..]",
                        fmt_bit(r.write), r.data_in.0, r.data_out.0),
                wiring::ComponentWiring::ROM(r) =>
                    write!(f, "rom[{}] addr=w{}[..] out=w{}[..]",
                        r.device_slot, r.addr.0, r.out.0),
                wiring::ComponentWiring::RAM(r) =>
                    write!(f, "ram[{}] addr=w{}[..] write={} in=w{}[..] out=w{}[..]",
                        r.device_slot, r.addr.0, fmt_bit(r.write), r.data_in.0, r.out.0),
                wiring::ComponentWiring::MemorySystem(m) =>
                    write!(f, "mem[{}] addr=w{}[..] write={} in=w{}[..] out=w{}[..]",
                        m.device_slot, m.addr.0, fmt_bit(m.write), m.data_in.0, m.out.0),
            }
        }
    }
    D(comp)
}

impl fmt::Display for ChipWiring {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut nands = 0u32;
        let mut muxes = 0u32;
        let mut registers = 0u32;
        for comp in &self.component_wiring {
            match comp {
                wiring::ComponentWiring::Nand(_)     => nands += 1,
                wiring::ComponentWiring::Mux(_)      => muxes += 1,
                wiring::ComponentWiring::Register(_) => registers += 1,
                _ => {}
            }
        }
        writeln!(f, "ChipWiring:")?;
        writeln!(f, "  wires:     {}", self.n_wires)?;
        writeln!(f, "  nands:     {}", nands)?;
        if muxes > 0 { writeln!(f, "  muxes:     {}", muxes)?; }
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

        let mut inputs: Vec<_> = self.input_wiring.iter().collect();
        inputs.sort_by_key(|(name, _)| name.clone());
        for (name, wr) in &inputs {
            writeln!(f, "  in  {name}: {}", fmt_wire(**wr))?;
        }
        let mut outputs: Vec<_> = self.output_wiring.iter().collect();
        outputs.sort_by_key(|(name, _)| name.clone());
        for (name, wr) in &outputs {
            writeln!(f, "  out {name}: {}", fmt_wire(**wr))?;
        }

        let mut consts: Vec<_> = self.const_wiring.iter().collect();
        for cw in &self.const_wiring {
            writeln!(f, "  const: w{} = {}", cw.out.0, cw.value)?;
        }

        for (i, comp) in self.component_wiring.iter().enumerate() {
            match comp {
                wiring::ComponentWiring::Nand(n) =>
                    writeln!(f, "  [{i}] nand  a={} b={} out={}",
                        fmt_bit(n.a), fmt_bit(n.b), fmt_bit(n.out))?,
                wiring::ComponentWiring::Mux(m) => {
                    writeln!(f, "  [{i}] mux   sel={} out=w{}[..]",
                        fmt_bit(m.sel), m.out.0)?;
                    writeln!(f, "         a0=w{}[..]", m.a0.0)?;
                    if m.branch0.is_empty() {
                        writeln!(f, "           <none>")?;
                    } else {
                        for op in &m.branch0 {
                            writeln!(f, "           {}", fmt_component(op))?;
                        }
                    }
                    writeln!(f, "         a1=w{}[..]", m.a1.0)?;
                    if m.branch1.is_empty() {
                        writeln!(f, "           <none>")?;
                    } else {
                        for op in &m.branch1 {
                            writeln!(f, "           {}", fmt_component(op))?;
                        }
                    }
                }
                wiring::ComponentWiring::Register(r) =>
                    writeln!(f, "  [{i}] reg   write={} in=w{}[..] out=w{}[..]",
                        fmt_bit(r.write), r.data_in.0, r.data_out.0)?,
                wiring::ComponentWiring::ROM(r) =>
                    writeln!(f, "  [{i}] rom[{}]  addr=w{}[..] out=w{}[..]",
                        r.device_slot, r.addr.0, r.out.0)?,
                wiring::ComponentWiring::RAM(r) =>
                    writeln!(f, "  [{i}] ram[{}]  addr=w{}[..] write={} in=w{}[..] out=w{}[..]",
                        r.device_slot, r.addr.0, fmt_bit(r.write), r.data_in.0, r.out.0)?,
                wiring::ComponentWiring::MemorySystem(m) =>
                    writeln!(f, "  [{i}] mem[{}]  addr=w{}[..] write={} in=w{}[..] out=w{}[..]",
                        m.device_slot, m.addr.0, fmt_bit(m.write), m.data_in.0, m.out.0)?,
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

    // Build map of wires that have been connected directly to some existing wire; when the Buffer's
    // "out" wire (the key in renamed) is encountered, the "a" wire should be substituted (the value
    // here)
    // Value is (bit offset for the src, WireID, bit offset of the dst)
    let mut renamed: HashMap<WireID, (usize, WireID, usize)> = HashMap::new();
    for comp in &components {
        match comp {
            Computational::Buffer(c) => {
                let intf = c.reflect();
                let a = &intf.inputs["a"];
                let out = &intf.outputs["out"];
                renamed.insert(WireID::from(out), (a.offset, WireID::from(a), out.offset));
            }
            _ => {}
        }
    }

    // Build wire_indexes: assign a contiguous WireIndex to every unique wire in the circuit.
    // This must be done before building component_wiring, which uses WireIndex directly.
    let mut wire_indexes: Indexes = HashMap::new();
    {
        let mut next_index = 0usize;
        let mut assign = |id: WireID| {
            if let Some(src) = renamed.get(&id) {
                // assign index to the src, insert id with that index
                let index: WireIndex;
                match wire_indexes.entry(src.1) {
                    Entry::Vacant(e) => {
                        index = WireIndex(next_index as u32);
                        e.insert(index);
                        next_index += 1;
                    }
                    Entry::Occupied(e) => {
                        index = *e.get();
                    }
                }
                wire_indexes.insert(id, index);
            }
            else if let Entry::Vacant(e) = wire_indexes.entry(id) {
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
                Computational::Const(c) => {
                    let intf = c.reflect();
                    assign(WireID::from(&intf.outputs["out"]));
                }
                Computational::Buffer(_) => {
                    // Ignore; already recorded in `renamed`
                }
                Computational::Mux(c) => {
                    let intf = c.reflect();
                    assign(WireID::from(&intf.inputs["a0"]));
                    assign(WireID::from(&intf.inputs["a1"]));
                    assign(WireID::from(&intf.inputs["sel"]));
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
            }
        }
    }

    let mut ram_specs: Vec<RAMSpec> = Vec::new();
    let mut rom_specs: Vec<ROMSpec> = Vec::new();
    let mut ms_specs:  Vec<MemorySystemSpec>  = Vec::new();

    let ref_for = |b: &BusRef| {
        let id = &WireID::from(b);
        if let Some((offset, src, _)) = renamed.get(id) {
            wiring::BitRef {
                id: wire_indexes[src],
                offset: *offset as u8,
            }
        }
        else {
            wiring::BitRef {
                id: wire_indexes[id],
                offset: b.offset as u8,
            }
        }
    };

    let const_wiring: Vec<wiring::ConstWiring> = components.iter().flat_map(|comp| {
        match comp {
            Computational::Const(c) => {
                let intf = c.reflect();
                Some(wiring::ConstWiring {
                    value: c.value,
                    out: wire_indexes[&WireID::from(&intf.outputs["out"])]
                })
            },
            _ => None,
        }
    }).collect();

    let component_wiring: Vec<wiring::ComponentWiring> = components.iter().flat_map(|comp| {
        use wiring::ComponentWiring as CW;
        match comp {
            Computational::Nand(c) => {
                let intf = c.reflect();
                Some(CW::Nand(wiring::NandWiring {
                    a:   ref_for(&intf.inputs["a"]),
                    b:   ref_for(&intf.inputs["b"]),
                    out: ref_for(&intf.outputs["out"]),
                }))
            }
            Computational::Const(_)        => None,
            Computational::Buffer(_)       => None,
            Computational::Mux(c)          => {
                let intf = c.reflect();
                Some(CW::Mux(wiring::MuxWiring {
                    sel: ref_for(&intf.inputs["sel"]),
                    a0:  wire_indexes[&WireID::from(&intf.inputs["a0"])],
                    a1:  wire_indexes[&WireID::from(&intf.inputs["a1"])],
                    out: wire_indexes[&WireID::from(&intf.outputs["out"])],
                    branch0: Vec::new(),
                    branch1: Vec::new(),
                }))
            }
            Computational::Register(c)     => {
                let intf = c.reflect();
                Some(CW::Register(wiring::RegisterWiring  {
                    write:    ref_for(&intf.inputs["write"]),
                    data_in:  wire_indexes[&WireID::from(&intf.inputs["data_in"])],
                    data_out: wire_indexes[&WireID::from(&intf.outputs["data_out"])],
                }))
            }
            Computational::RAM(c)          => {
                let slot = ram_specs.len();
                ram_specs.push(RAMSpec { size: c.size });

                let intf = c.reflect();
                Some(CW::RAM(wiring::RAMWiring {
                    device_slot: slot,
                    out:     wire_indexes[&WireID::from(&intf.outputs["data_out"])],
                    addr:    wire_indexes[&WireID::from(&intf.inputs["addr"])],
                    write:   ref_for(&intf.inputs["write"]),
                    data_in: wire_indexes[&WireID::from(&intf.inputs["data_in"])],
                }))
            }
            Computational::ROM(c)          => {
                let slot = rom_specs.len();
                rom_specs.push(ROMSpec { size: c.size });

                let intf = c.reflect();
                Some(CW::ROM(wiring::ROMWiring{
                    device_slot: slot,
                    out:  wire_indexes[&WireID::from(&intf.outputs["out"])],
                    addr: wire_indexes[&WireID::from(&intf.inputs["addr"])],
                }))
            }
            Computational::MemorySystem(c) => {
                let slot = ms_specs.len();
                let regions = memory_map.take().expect("only one MemorySystem supported").contents;
                ms_specs.push(MemorySystemSpec { regions });

                let intf = c.reflect();
                Some(CW::MemorySystem(wiring::MemorySystemWiring {
                    device_slot: slot,
                    out:     wire_indexes[&WireID::from(&intf.outputs["data_out"])],
                    addr:    wire_indexes[&WireID::from(&intf.inputs["addr"])],
                    write:   ref_for(&intf.inputs["write"]),
                    data_in: wire_indexes[&WireID::from(&intf.inputs["data_in"])],
                }))
            }
        }
    }).collect();

    let n_wires = wire_indexes.len();
    let intf = chip.reflect();

    let to_wr = |(name, b): (&String, &BusRef)| {
        if let Some((offset, _, _)) = renamed.get(&WireID::from(b)) {
           (name.clone(),
            WireRef {
                id: wire_indexes[&WireID::from(b)],
                offset: *offset as u8,
                width: b.width as u8
            })
       }
        else {
            (name.clone(),
            WireRef {
                id: wire_indexes[&WireID::from(b)],
                offset: b.offset as u8,
                width: b.width as u8
            })
        }
    };
    ChipWiring {
        component_wiring,
        input_wiring:  intf.inputs.iter().map(to_wr).collect(),
        output_wiring: intf.outputs.iter().map(to_wr).collect(),
        const_wiring,
        n_wires,
        ram_specs,
        rom_specs,
        ms_specs,
    }
}
