use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::fmt;
use std::marker::PhantomData;

use crate::component::Computational;
use crate::component::native::Simulational;
use crate::declare::{BusRef, IC, Reflect as _};
use crate::nat::Nat;
use crate::word::{Storable, Word};

use super::memory::{MemoryMap, RegionMap};
use super::wiring::{self, Indexes, WireID, WireIndex, WireRef};

/// Static, synthesized description of the circuit's wiring. Computed once and never mutated.
pub struct ChipWiring<Width: Storable> {
    pub(super) component_wiring: Vec<wiring::ComponentWiring>,
    pub(super) input_wiring: HashMap<String, wiring::WireRef>,
    pub(super) output_wiring: HashMap<String, wiring::WireRef>,
    pub(super) const_wiring: Vec<wiring::ConstWiring>,
    pub(super) n_wires: usize,

    /// One entry per RAM component; the index is the device slot referenced by the wiring.
    pub ram_specs: Vec<RAMSpec>,
    /// One entry per ROM component; the index is the device slot referenced by the wiring.
    pub rom_specs: Vec<ROMSpec>,
    /// One entry per Serial I/O component.
    pub serial_specs: Vec<SerialSpec>,
    /// One entry per MemorySystem component, including the RAM region layout.
    pub ms_specs: Vec<MemorySystemSpec>,

    _width: PhantomData<Width>,
}

/// Descriptor for a standalone RAM component.
pub struct RAMSpec {
    pub size: usize,
}

/// Descriptor for a ROM component.
pub struct ROMSpec {
    pub size: usize,
}

/// Descriptor for a Serial I/O component.
pub struct SerialSpec;

/// Descriptor for a MemorySystem component, including its region layout.
pub struct MemorySystemSpec {
    pub regions: Vec<RegionMap>,
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
            } else {
                write!(
                    f,
                    "w{}[{}..{}]",
                    self.0.id.0,
                    self.0.offset,
                    self.0.offset + self.0.width
                )
            }
        }
    }
    D(wr)
}

fn fmt_component_tree(
    f: &mut fmt::Formatter<'_>,
    comp: &wiring::ComponentWiring,
    indent: &str,
) -> fmt::Result {
    match comp {
        wiring::ComponentWiring::Nand(n) if n.a == n.b => {
            writeln!(f, "not   a={} out={}", fmt_bit(n.a), fmt_bit(n.out))
        }
        wiring::ComponentWiring::Nand(n) => writeln!(
            f,
            "nand  a={} b={} out={}",
            fmt_bit(n.a),
            fmt_bit(n.b),
            fmt_bit(n.out)
        ),
        wiring::ComponentWiring::Mux(m) => {
            writeln!(f, "mux   sel={} out=w{}[..]", fmt_bit(m.sel), m.out.0)?;
            let c0 = count_components(&m.branch0);
            let c1 = count_components(&m.branch1);
            if c0.adders.0 > 0 {
                writeln!(
                    f,
                    "{indent}     a0=w{}[..] ({} gates, {} adders)",
                    m.a0.0, c0.gates.0, c0.adders.0
                )?;
            } else {
                writeln!(f, "{indent}     a0=w{}[..] ({} gates)", m.a0.0, c0.gates.0)?;
            }
            let inner = format!("{indent}       ");
            for op in &m.branch0 {
                write!(f, "{inner}")?;
                fmt_component_tree(f, op, &inner)?;
            }
            if c1.adders.0 > 0 {
                writeln!(
                    f,
                    "{indent}     a1=w{}[..] ({} gates, {} adders)",
                    m.a1.0, c1.gates.0, c1.adders.0
                )?;
            } else {
                writeln!(f, "{indent}     a1=w{}[..] ({} gates)", m.a1.0, c1.gates.0)?;
            }
            for op in &m.branch1 {
                write!(f, "{inner}")?;
                fmt_component_tree(f, op, &inner)?;
            }
            Ok(())
        }
        wiring::ComponentWiring::Adder(a) => writeln!(
            f,
            "adder a={} b={} c={} sum={} carry={}",
            fmt_bit(a.a),
            fmt_bit(a.b),
            fmt_bit(a.c),
            fmt_bit(a.sum),
            fmt_bit(a.carry)
        ),

        wiring::ComponentWiring::Register(r) => writeln!(
            f,
            "reg   write={} in=w{}[..] out=w{}[..]",
            fmt_bit(r.write),
            r.data_in.0,
            r.data_out.0
        ),

        wiring::ComponentWiring::RAM(r) => writeln!(
            f,
            "ram[{}]  addr=w{}[..] write={} in=w{}[..] out=w{}[..]",
            r.device_slot,
            r.addr.0,
            fmt_bit(r.write),
            r.data_in.0,
            r.out.0
        ),
        wiring::ComponentWiring::ROM(r) => writeln!(
            f,
            "rom[{}]  addr=w{}[..] out=w{}[..]",
            r.device_slot, r.addr.0, r.out.0
        ),
        wiring::ComponentWiring::Serial(s) => writeln!(
            f,
            "serial[{}]  write={} in=w{}[..] out=w{}[..]",
            s.device_slot,
            fmt_bit(s.write),
            s.data_in.0,
            s.out.0
        ),
        wiring::ComponentWiring::MemorySystem(m) => writeln!(
            f,
            "mem[{}]  addr=w{}[..] write={} in=w{}[..] out=w{}[..]",
            m.device_slot,
            m.addr.0,
            fmt_bit(m.write),
            m.data_in.0,
            m.out.0
        ),

        // synthetic:
        wiring::ComponentWiring::And(n) => writeln!(
            f,
            "and   a={} b={} out={}",
            fmt_bit(n.a),
            fmt_bit(n.b),
            fmt_bit(n.out)
        ),

        wiring::ComponentWiring::ParallelNand(n) if n.a == n.b => {
            writeln!(f, "not(many) a=w{}[..] out=w{}[..]", n.a.0, n.out.0)
        }
        wiring::ComponentWiring::ParallelNand(n) => writeln!(
            f,
            "nand(many) a=w{}[..] b=w{}[..] out=w{}[..]",
            n.a.0, n.b.0, n.out.0
        ),

        wiring::ComponentWiring::RippleAdder(a) => writeln!(
            f,
            "adder(ripple) a=w{}[{}..{}] b=w{}[..] out=w{}[{}..{}] carry_in={} carry_out={}",
            a.a.0,
            a.offset,
            a.offset + a.width,
            a.b.0,
            a.out.0,
            a.offset,
            a.offset + a.width,
            fmt_bit(a.carry_in),
            fmt_bit(a.carry_out)
        ),

        wiring::ComponentWiring::ManyWayAnd(m) => writeln!(
            f,
            "and(many-way) a=w{}[..] mask={:#06x} out={}",
            m.a.0,
            m.mask,
            fmt_bit(m.out)
        ),
    }
}

/// Counts for gates and adders in a component list, recursing into mux branches.
/// Each field is (total, min) where min assumes the cheaper branch at each mux.
struct ComponentCounts {
    gates: (u32, u32),
    adders: (u32, u32),
}

fn count_components(components: &[wiring::ComponentWiring]) -> ComponentCounts {
    let mut gates = (0u32, 0u32);
    let mut adders = (0u32, 0u32);
    for comp in components {
        match comp {
            wiring::ComponentWiring::Nand(_)
            | wiring::ComponentWiring::And(_)
            | wiring::ComponentWiring::ParallelNand(_)
            | wiring::ComponentWiring::ManyWayAnd(_) => {
                gates.0 += 1;
                gates.1 += 1;
            }
            wiring::ComponentWiring::Adder(_) | wiring::ComponentWiring::RippleAdder(_) => {
                adders.0 += 1;
                adders.1 += 1;
            }
            wiring::ComponentWiring::Mux(m) => {
                let c0 = count_components(&m.branch0);
                let c1 = count_components(&m.branch1);
                gates.0 += c0.gates.0 + c1.gates.0;
                gates.1 += c0.gates.1.min(c1.gates.1);
                adders.0 += c0.adders.0 + c1.adders.0;
                adders.1 += c0.adders.1.min(c1.adders.1);
            }
            _ => {}
        }
    }
    ComponentCounts { gates, adders }
}

impl<Width: Storable> fmt::Display for ChipWiring<Width> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut muxes = 0u32;
        let mut registers = 0u32;
        for comp in &self.component_wiring {
            match comp {
                wiring::ComponentWiring::Mux(_) => muxes += 1,
                wiring::ComponentWiring::Register(_) => registers += 1,
                _ => {}
            }
        }
        let counts = count_components(&self.component_wiring);
        let (total_gates, min_gates) = counts.gates;
        let (total_adders, min_adders) = counts.adders;
        writeln!(f, "ChipWiring:")?;
        write!(f, "  gates:     {} total", total_gates)?;
        if min_gates < total_gates {
            writeln!(f, ", {} min/cycle", min_gates)?;
        } else {
            writeln!(f)?;
        }
        if total_adders > 0 {
            write!(f, "  adders:    {} total", total_adders)?;
            if min_adders < total_adders {
                writeln!(f, ", {} min/cycle", min_adders)?;
            } else {
                writeln!(f)?;
            }
        }
        if muxes > 0 {
            writeln!(f, "  muxes:     {}", muxes)?;
        }
        if registers > 0 {
            writeln!(f, "  registers: {}", registers)?;
        }
        for (i, s) in self.ram_specs.iter().enumerate() {
            writeln!(f, "  ram[{}]:    {} words", i, s.size)?;
        }
        for (i, s) in self.rom_specs.iter().enumerate() {
            writeln!(f, "  rom[{}]:    {} words", i, s.size)?;
        }
        for (i, _) in self.serial_specs.iter().enumerate() {
            writeln!(f, "  serial[{}]", i)?;
        }
        for (i, ms) in self.ms_specs.iter().enumerate() {
            writeln!(f, "  memory[{}]:", i)?;
            for r in &ms.regions {
                match r {
                    RegionMap::RAM(m) => {
                        writeln!(f, "    RAM: {} words @ 0x{:04x}", m.size, m.base)?
                    }
                    RegionMap::ROM(m) => {
                        writeln!(f, "    ROM: {} words @ 0x{:04x}", m.size, m.base)?
                    }
                    RegionMap::Serial(m) => writeln!(f, "    Serial @ 0x{:04x}", m.base)?,
                }
            }
        }

        let mut inputs: Vec<_> = self.input_wiring.iter().collect();
        inputs.sort_by_key(|(name, _)| *name);
        for (name, wr) in &inputs {
            writeln!(f, "  in  {name}: {}", fmt_wire(**wr))?;
        }
        let mut outputs: Vec<_> = self.output_wiring.iter().collect();
        outputs.sort_by_key(|(name, _)| *name);
        for (name, wr) in &outputs {
            writeln!(f, "  out {name}: {}", fmt_wire(**wr))?;
        }

        for cw in &self.const_wiring {
            writeln!(
                f,
                "  const: w{} = {}",
                cw.out.0,
                Word::<Width>::new(cw.value)
            )?;
        }

        for (i, comp) in self.component_wiring.iter().enumerate() {
            write!(f, "  [{i}] ")?;
            fmt_component_tree(f, comp, "  ")?;
        }
        Ok(())
    }
}

/// Counts of each top-level operation type in a synthesized wiring.
#[derive(Debug, Default)]
pub struct OpCounts {
    pub nands: usize,
    pub ands: usize,
    pub adders: usize,
    pub muxes: usize,
    pub parallel_nands: usize,
    pub ripple_adders: usize,
    pub many_way_ands: usize,
    pub registers: usize,
}

impl<Width: Storable> ChipWiring<Width> {
    /// Count top-level operations by type (does not recurse into mux branches).
    pub fn op_counts(&self) -> OpCounts {
        let mut c = OpCounts::default();
        for comp in &self.component_wiring {
            match comp {
                wiring::ComponentWiring::Nand(_) => c.nands += 1,
                wiring::ComponentWiring::And(_) => c.ands += 1,
                wiring::ComponentWiring::Adder(_) => c.adders += 1,
                wiring::ComponentWiring::Mux(_) => c.muxes += 1,
                wiring::ComponentWiring::ParallelNand(_) => c.parallel_nands += 1,
                wiring::ComponentWiring::RippleAdder(_) => c.ripple_adders += 1,
                wiring::ComponentWiring::ManyWayAnd(_) => c.many_way_ands += 1,
                wiring::ComponentWiring::Register(_) => c.registers += 1,
                _ => {}
            }
        }
        c
    }
}

/// Transform a circuit description into a pre-computed wiring layout.
///
/// No RAM or ROM buffers are allocated here. Call [`super::initialize`] to create a runnable
/// [`super::ChipState`].
///
/// Note: currently 16-bit words are assumed, but up to 64-bits wouldn't be a problem if the type
/// was generalized.
pub fn synthesize<C, A: Nat + Storable + Clone, D: Nat + Storable + Clone>(
    chip: &IC<C>,
    memory_map: MemoryMap,
) -> ChipWiring<D>
where
    C: Clone + crate::Reflect + Into<Simulational<A, D>>,
{
    let components: Vec<Simulational<A, D>> =
        chip.components.iter().cloned().map(Into::into).collect();
    let mut memory_map = Some(memory_map);

    // Build map of wires that have been connected directly to some existing wire; when the Buffer's
    // "out" wire (the key in renamed) is encountered, the "a" wire should be substituted (the value
    // here). Only full-bus Buffers are eligible for renaming; sub-bus Buffers (e.g. bit(0) of a
    // wider bus) must emit a copy op instead, since renaming the wire ID would alias the entire bus.
    // Value is (bit offset for the src, WireID, bit offset of the dst)
    let mut renamed: HashMap<WireID, (usize, WireID, usize)> = HashMap::new();
    // Buffers that connect a single bit of a wider bus — need a copy op instead of rename.
    let mut sub_bus_buffers: Vec<usize> = Vec::new();
    for (idx, comp) in components.iter().enumerate() {
        match comp {
            Simulational::Primitive(Computational::Buffer(c)) => {
                let intf = c.reflect();
                let a = &intf.inputs["a"];
                let out = &intf.outputs["out"];
                if a.width > 1 && out.width > 1 {
                    renamed.insert(WireID::from(out), (a.offset, WireID::from(a), out.offset));
                } else {
                    sub_bus_buffers.push(idx);
                }
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
            } else if let Entry::Vacant(e) = wire_indexes.entry(id) {
                e.insert(WireIndex(next_index as u32));
                next_index += 1;
            }
        };

        let intf = chip.reflect();
        for b in intf.inputs.values() {
            assign(WireID::from(b));
        }
        for b in intf.outputs.values() {
            assign(WireID::from(b));
        }
        for comp in &components {
            match comp {
                Simulational::Primitive(Computational::Nand(c)) => {
                    let intf = c.reflect();
                    assign(WireID::from(&intf.inputs["a"]));
                    assign(WireID::from(&intf.inputs["b"]));
                    assign(WireID::from(&intf.outputs["out"]));
                }
                Simulational::Primitive(Computational::Buffer(c)) => {
                    // Full-bus buffers are handled via `renamed`; sub-bus buffers need their
                    // `a` wire assigned explicitly (including the case where `a` is fixed).
                    let intf = c.reflect();
                    let a = &intf.inputs["a"];
                    if a.width == 1 {
                        assign(WireID::from(a));
                        assign(WireID::from(&intf.outputs["out"]));
                    }
                }
                Simulational::Mux(c) => {
                    let intf = c.reflect();
                    assign(WireID::from(&intf.inputs["a0"]));
                    assign(WireID::from(&intf.inputs["a1"]));
                    assign(WireID::from(&intf.inputs["sel"]));
                    assign(WireID::from(&intf.outputs["out"]));
                }
                Simulational::Adder(c) => {
                    let intf = c.reflect();
                    assign(WireID::from(&intf.inputs["a"]));
                    assign(WireID::from(&intf.inputs["b"]));
                    assign(WireID::from(&intf.inputs["c"]));
                    assign(WireID::from(&intf.outputs["sum"]));
                    assign(WireID::from(&intf.outputs["carry"]));
                }

                Simulational::Primitive(Computational::Register(c)) => {
                    let intf = c.reflect();
                    assign(WireID::from(&intf.inputs["write"]));
                    assign(WireID::from(&intf.inputs["data_in"]));
                    assign(WireID::from(&intf.outputs["data_out"]));
                }
                Simulational::Primitive(Computational::RAM(c)) => {
                    let intf = c.reflect();
                    assign(WireID::from(&intf.outputs["data_out"]));
                    assign(WireID::from(&intf.inputs["addr"]));
                    assign(WireID::from(&intf.inputs["write"]));
                    assign(WireID::from(&intf.inputs["data_in"]));
                }
                Simulational::Primitive(Computational::ROM(c)) => {
                    let intf = c.reflect();
                    assign(WireID::from(&intf.outputs["out"]));
                    assign(WireID::from(&intf.inputs["addr"]));
                }
                Simulational::Primitive(Computational::Serial(c)) => {
                    let intf = c.reflect();
                    assign(WireID::from(&intf.outputs["data_out"]));
                    assign(WireID::from(&intf.inputs["write"]));
                    assign(WireID::from(&intf.inputs["data_in"]));
                }
                Simulational::Primitive(Computational::MemorySystem(c)) => {
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
    let mut serial_specs: Vec<SerialSpec> = Vec::new();
    let mut ms_specs: Vec<MemorySystemSpec> = Vec::new();

    let ref_for = |b: &BusRef| {
        let id = &WireID::from(b);
        if let Some((offset, src, _)) = renamed.get(id) {
            wiring::BitRef {
                id: *wire_indexes.get(src).unwrap_or_else(|| {
                    panic!(
                        "ref_for: renamed wire source not in wire_indexes (width={}, offset={})",
                        b.width, b.offset
                    )
                }),
                offset: *offset as u8,
            }
        } else {
            wiring::BitRef {
                id: *wire_indexes.get(id).unwrap_or_else(|| {
                    panic!(
                        "ref_for: wire not in wire_indexes (width={}, offset={})",
                        b.width, b.offset
                    )
                }),
                offset: b.offset as u8,
            }
        }
    };

    let const_wiring: Vec<wiring::ConstWiring> = components
        .iter()
        .flat_map(|comp| {
            comp.reflect()
                .inputs
                .values()
                .filter_map(|busref| {
                    busref.fixed.map(|value| wiring::ConstWiring {
                        value,
                        out: wire_indexes[&WireID::from(busref)],
                    })
                })
                .collect::<Vec<_>>()
        })
        .collect();

    let component_wiring: Vec<wiring::ComponentWiring> = components
        .iter()
        .enumerate()
        .flat_map(|(idx, comp)| {
            use wiring::ComponentWiring as CW;
            match comp {
                Simulational::Primitive(Computational::Nand(c)) => {
                    let intf = c.reflect();
                    Some(CW::Nand(wiring::NandWiring {
                        a: ref_for(&intf.inputs["a"]),
                        b: ref_for(&intf.inputs["b"]),
                        out: ref_for(&intf.outputs["out"]),
                    }))
                }
                Simulational::Primitive(Computational::Buffer(c)) => {
                    if sub_bus_buffers.contains(&idx) {
                        // Sub-bus buffer: emit And(x,x) as a single-bit copy.
                        let intf = c.reflect();
                        let a = ref_for(&intf.inputs["a"]);
                        let out = ref_for(&intf.outputs["out"]);
                        Some(CW::And(wiring::AndWiring { a, b: a, out }))
                    } else {
                        None // Full-bus buffer: handled by rename
                    }
                }
                Simulational::Mux(c) => {
                    let intf = c.reflect();
                    Some(CW::Mux(wiring::MuxWiring {
                        sel: ref_for(&intf.inputs["sel"]),
                        a0: wire_indexes[&WireID::from(&intf.inputs["a0"])],
                        a1: wire_indexes[&WireID::from(&intf.inputs["a1"])],
                        out: wire_indexes[&WireID::from(&intf.outputs["out"])],
                        branch0: Vec::new(),
                        branch1: Vec::new(),
                    }))
                }
                Simulational::Adder(c) => {
                    let intf = c.reflect();
                    Some(CW::Adder(wiring::AdderWiring {
                        a: ref_for(&intf.inputs["a"]),
                        b: ref_for(&intf.inputs["b"]),
                        c: ref_for(&intf.inputs["c"]),
                        sum: ref_for(&intf.outputs["sum"]),
                        carry: ref_for(&intf.outputs["carry"]),
                    }))
                }
                Simulational::Primitive(Computational::Register(c)) => {
                    let intf = c.reflect();
                    Some(CW::Register(wiring::RegisterWiring {
                        write: ref_for(&intf.inputs["write"]),
                        data_in: wire_indexes[&WireID::from(&intf.inputs["data_in"])],
                        data_out: wire_indexes[&WireID::from(&intf.outputs["data_out"])],
                    }))
                }
                Simulational::Primitive(Computational::RAM(c)) => {
                    let slot = ram_specs.len();
                    ram_specs.push(RAMSpec { size: c.size });

                    let intf = c.reflect();
                    Some(CW::RAM(wiring::RAMWiring {
                        device_slot: slot,
                        out: wire_indexes[&WireID::from(&intf.outputs["data_out"])],
                        addr: wire_indexes[&WireID::from(&intf.inputs["addr"])],
                        write: ref_for(&intf.inputs["write"]),
                        data_in: wire_indexes[&WireID::from(&intf.inputs["data_in"])],
                    }))
                }
                Simulational::Primitive(Computational::ROM(c)) => {
                    let slot = rom_specs.len();
                    rom_specs.push(ROMSpec { size: c.size });

                    let intf = c.reflect();
                    Some(CW::ROM(wiring::ROMWiring {
                        device_slot: slot,
                        out: wire_indexes[&WireID::from(&intf.outputs["out"])],
                        addr: wire_indexes[&WireID::from(&intf.inputs["addr"])],
                    }))
                }
                Simulational::Primitive(Computational::Serial(c)) => {
                    let slot = serial_specs.len();
                    serial_specs.push(SerialSpec);

                    let intf = c.reflect();
                    Some(CW::Serial(wiring::SerialWiring {
                        device_slot: slot,
                        out: wire_indexes[&WireID::from(&intf.outputs["data_out"])],
                        write: ref_for(&intf.inputs["write"]),
                        data_in: wire_indexes[&WireID::from(&intf.inputs["data_in"])],
                    }))
                }
                Simulational::Primitive(Computational::MemorySystem(c)) => {
                    let slot = ms_specs.len();
                    let regions = memory_map
                        .take()
                        .expect("only one MemorySystem supported")
                        .regions;
                    ms_specs.push(MemorySystemSpec { regions });

                    let intf = c.reflect();
                    Some(CW::MemorySystem(wiring::MemorySystemWiring {
                        device_slot: slot,
                        out: wire_indexes[&WireID::from(&intf.outputs["data_out"])],
                        addr: wire_indexes[&WireID::from(&intf.inputs["addr"])],
                        write: ref_for(&intf.inputs["write"]),
                        data_in: wire_indexes[&WireID::from(&intf.inputs["data_in"])],
                    }))
                }
            }
        })
        .collect();

    // Peephole: collapse nand+not into and.
    let mut component_wiring = component_wiring;
    let output_wires: Vec<wiring::WireIndex> = chip
        .reflect()
        .outputs
        .values()
        .map(|b| wire_indexes[&WireID::from(b)])
        .collect();
    peephole_nand_not(&mut component_wiring, &output_wires);

    // Remove gates whose output is never consumed.
    eliminate_dead_gates(&mut component_wiring, &output_wires);

    // Coalesce bit-parallel operations into single word-wide ops.
    coalesce_parallel_nands(&mut component_wiring);
    coalesce_many_way_ands(&mut component_wiring);
    let zero_wires: std::collections::HashSet<wiring::WireIndex> = const_wiring
        .iter()
        .filter(|c| c.value == 0)
        .map(|c| c.out)
        .collect();
    coalesce_ripple_adders(&mut component_wiring, &zero_wires);

    // Post-processing: move exclusive producers into mux branches (recursively).
    populate_mux_branches(&mut component_wiring, &output_wires);

    // Topological sort: ensure evaluation order respects dependencies.
    topo_sort_wiring(&mut component_wiring);

    let n_wires = wire_indexes.len();
    let intf = chip.reflect();

    let to_wr = |(name, b): (&String, &BusRef)| {
        if let Some((offset, _, _)) = renamed.get(&WireID::from(b)) {
            (
                name.clone(),
                WireRef {
                    id: wire_indexes[&WireID::from(b)],
                    offset: *offset as u8,
                    width: b.width as u8,
                },
            )
        } else {
            (
                name.clone(),
                WireRef {
                    id: wire_indexes[&WireID::from(b)],
                    offset: b.offset as u8,
                    width: b.width as u8,
                },
            )
        }
    };
    ChipWiring {
        component_wiring,
        input_wiring: intf.inputs.iter().map(to_wr).collect(),
        output_wiring: intf.outputs.iter().map(to_wr).collect(),
        const_wiring,
        n_wires,
        ram_specs,
        rom_specs,
        serial_specs,
        ms_specs,
        _width: PhantomData,
    }
}

/// Peephole: replace `nand a,b -> n; nand n,n -> out` with `and a,b -> out`.
/// The not (self-nand) must be the sole consumer of the nand's output.
fn peephole_nand_not(
    components: &mut Vec<wiring::ComponentWiring>,
    output_wires: &[wiring::WireIndex],
) {
    use std::collections::{HashMap as Map, HashSet};
    use wiring::ComponentWiring as CW;

    // For each wire bit, track the set of distinct component indices that consume it.
    // Also track wires consumed as a bus (WireIndex) — those can't be optimized.
    let mut wire_consumers: Map<(u32, u8), HashSet<usize>> = Map::new();
    let mut bus_consumed: HashSet<u32> = HashSet::new();
    // Chip-level outputs consume entire wires.
    for w in output_wires {
        bus_consumed.insert(w.0);
    }
    for (i, comp) in components.iter().enumerate() {
        match comp {
            CW::Nand(n) => {
                wire_consumers
                    .entry((n.a.id.0, n.a.offset))
                    .or_default()
                    .insert(i);
                wire_consumers
                    .entry((n.b.id.0, n.b.offset))
                    .or_default()
                    .insert(i);
            }
            CW::And(n) => {
                wire_consumers
                    .entry((n.a.id.0, n.a.offset))
                    .or_default()
                    .insert(i);
                wire_consumers
                    .entry((n.b.id.0, n.b.offset))
                    .or_default()
                    .insert(i);
            }
            CW::ParallelNand(n) => {
                bus_consumed.insert(n.a.0);
                bus_consumed.insert(n.b.0);
            }
            CW::RippleAdder(a) => {
                bus_consumed.insert(a.a.0);
                bus_consumed.insert(a.b.0);
                wire_consumers
                    .entry((a.carry_in.id.0, a.carry_in.offset))
                    .or_default()
                    .insert(i);
            }
            CW::ManyWayAnd(m) => {
                bus_consumed.insert(m.a.0);
            }
            CW::Mux(m) => {
                wire_consumers
                    .entry((m.sel.id.0, m.sel.offset))
                    .or_default()
                    .insert(i);
                bus_consumed.insert(m.a0.0);
                bus_consumed.insert(m.a1.0);
            }
            CW::Adder(a) => {
                wire_consumers
                    .entry((a.a.id.0, a.a.offset))
                    .or_default()
                    .insert(i);
                wire_consumers
                    .entry((a.b.id.0, a.b.offset))
                    .or_default()
                    .insert(i);
                wire_consumers
                    .entry((a.c.id.0, a.c.offset))
                    .or_default()
                    .insert(i);
            }
            CW::Register(r) => {
                wire_consumers
                    .entry((r.write.id.0, r.write.offset))
                    .or_default()
                    .insert(i);
                bus_consumed.insert(r.data_in.0);
            }
            CW::RAM(r) => {
                wire_consumers
                    .entry((r.write.id.0, r.write.offset))
                    .or_default()
                    .insert(i);
                bus_consumed.insert(r.data_in.0);
                bus_consumed.insert(r.addr.0);
            }
            CW::ROM(r) => {
                bus_consumed.insert(r.addr.0);
            }
            CW::Serial(s) => {
                wire_consumers
                    .entry((s.write.id.0, s.write.offset))
                    .or_default()
                    .insert(i);
                bus_consumed.insert(s.data_in.0);
            }
            CW::MemorySystem(m) => {
                wire_consumers
                    .entry((m.write.id.0, m.write.offset))
                    .or_default()
                    .insert(i);
                bus_consumed.insert(m.data_in.0);
                bus_consumed.insert(m.addr.0);
            }
        }
    }

    // Build a map from nand output BitRef -> index, for nands whose output has exactly
    // one consumer component AND whose output wire is not bus-consumed.
    let mut nand_by_out: Map<(u32, u8), usize> = Map::new();
    for (i, comp) in components.iter().enumerate() {
        if let CW::Nand(n) = comp {
            let key = (n.out.id.0, n.out.offset);
            if bus_consumed.contains(&n.out.id.0) {
                continue;
            }
            if let Some(consumers) = wire_consumers.get(&key) {
                if consumers.len() == 1 {
                    nand_by_out.insert(key, i);
                }
            }
        }
    }

    // Find not gates (nand with a==b) whose input comes from a single-consumer nand.
    // Replace the pair with a single And gate.
    let mut to_remove: HashSet<usize> = HashSet::new();
    for i in 0..components.len() {
        if to_remove.contains(&i) {
            continue;
        }
        let CW::Nand(not_gate) = &components[i] else {
            continue;
        };
        if not_gate.a != not_gate.b {
            continue;
        }
        let key = (not_gate.a.id.0, not_gate.a.offset);
        let Some(&nand_idx) = nand_by_out.get(&key) else {
            continue;
        };
        if nand_idx == i {
            continue;
        }
        if to_remove.contains(&nand_idx) {
            continue;
        }
        // nand_idx may have already been converted to And by a prior iteration.
        let CW::Nand(nand_gate) = &components[nand_idx] else {
            continue;
        };
        let and = wiring::AndWiring {
            a: nand_gate.a,
            b: nand_gate.b,
            out: not_gate.out,
        };
        components[nand_idx] = CW::And(and);
        to_remove.insert(i);
    }

    // Remove the consumed not gates (in reverse order to preserve indices).
    let mut to_remove_sorted: Vec<usize> = to_remove.into_iter().collect();
    to_remove_sorted.sort_unstable();
    for &i in to_remove_sorted.iter().rev() {
        components.remove(i);
    }
}

/// Remove gates (Nand/And) whose output wire has no consumers at all.
/// Iterates to a fixed point, since removing a gate may leave its inputs' producers dead too.
fn eliminate_dead_gates(
    components: &mut Vec<wiring::ComponentWiring>,
    output_wires: &[wiring::WireIndex],
) {
    use std::collections::HashSet;
    use wiring::ComponentWiring as CW;

    loop {
        // Collect all consumed wire bits and bus-consumed wires.
        let mut consumed_bits: HashSet<(u32, u8)> = HashSet::new();
        let mut bus_consumed: HashSet<u32> = HashSet::new();
        for w in output_wires {
            bus_consumed.insert(w.0);
        }
        for comp in components.iter() {
            match comp {
                CW::Nand(n) => {
                    consumed_bits.insert((n.a.id.0, n.a.offset));
                    consumed_bits.insert((n.b.id.0, n.b.offset));
                }
                CW::And(n) => {
                    consumed_bits.insert((n.a.id.0, n.a.offset));
                    consumed_bits.insert((n.b.id.0, n.b.offset));
                }
                CW::ParallelNand(n) => {
                    bus_consumed.insert(n.a.0);
                    bus_consumed.insert(n.b.0);
                }
                CW::RippleAdder(a) => {
                    bus_consumed.insert(a.a.0);
                    bus_consumed.insert(a.b.0);
                    consumed_bits.insert((a.carry_in.id.0, a.carry_in.offset));
                }
                CW::ManyWayAnd(m) => {
                    bus_consumed.insert(m.a.0);
                }
                CW::Mux(m) => {
                    consumed_bits.insert((m.sel.id.0, m.sel.offset));
                    bus_consumed.insert(m.a0.0);
                    bus_consumed.insert(m.a1.0);
                }
                CW::Adder(a) => {
                    consumed_bits.insert((a.a.id.0, a.a.offset));
                    consumed_bits.insert((a.b.id.0, a.b.offset));
                    consumed_bits.insert((a.c.id.0, a.c.offset));
                }
                CW::Register(r) => {
                    consumed_bits.insert((r.write.id.0, r.write.offset));
                    bus_consumed.insert(r.data_in.0);
                }
                CW::RAM(r) => {
                    consumed_bits.insert((r.write.id.0, r.write.offset));
                    bus_consumed.insert(r.data_in.0);
                    bus_consumed.insert(r.addr.0);
                }
                CW::ROM(r) => {
                    bus_consumed.insert(r.addr.0);
                }
                CW::Serial(s) => {
                    consumed_bits.insert((s.write.id.0, s.write.offset));
                    bus_consumed.insert(s.data_in.0);
                }
                CW::MemorySystem(m) => {
                    consumed_bits.insert((m.write.id.0, m.write.offset));
                    bus_consumed.insert(m.data_in.0);
                    bus_consumed.insert(m.addr.0);
                }
            }
        }

        let before = components.len();
        components.retain(|comp| {
            match comp {
                CW::Nand(n) => {
                    bus_consumed.contains(&n.out.id.0)
                        || consumed_bits.contains(&(n.out.id.0, n.out.offset))
                }
                CW::And(n) => {
                    bus_consumed.contains(&n.out.id.0)
                        || consumed_bits.contains(&(n.out.id.0, n.out.offset))
                }
                CW::ManyWayAnd(m) => {
                    bus_consumed.contains(&m.out.id.0)
                        || consumed_bits.contains(&(m.out.id.0, m.out.offset))
                }
                _ => true, // keep registers, RAM, ROM, muxes, memory systems
            }
        });

        if components.len() == before {
            break;
        }
    }
}

/// Coalesce consecutive `NandWiring` or `AndWiring` entries that operate on different bits of the
/// same wires into parallel operations. Each individual gate must use the same bit offset for its
/// a, b, and out refs (i.e. bit N of a op'd with bit N of b producing bit N of out).
///
/// - A run of Nands becomes one `ParallelNandWiring`.
/// - A run of Ands becomes two `ParallelNandWiring`s: nand into out, then not (self-nand) in place.
fn coalesce_parallel_nands(components: &mut Vec<wiring::ComponentWiring>) {
    use wiring::ComponentWiring as CW;

    // Key: (a wire, b wire, out wire). Gates with the same key and aligned offsets can merge.
    type GroupKey = (u32, u32, u32);

    fn aligned_key(
        a: &wiring::BitRef,
        b: &wiring::BitRef,
        out: &wiring::BitRef,
    ) -> Option<GroupKey> {
        if a.offset == b.offset && b.offset == out.offset {
            Some((a.id.0, b.id.0, out.id.0))
        } else {
            None
        }
    }

    #[derive(PartialEq, Clone, Copy)]
    enum RunKind {
        Nand,
        And,
    }

    let mut result: Vec<CW> = Vec::with_capacity(components.len());
    let mut i = 0;
    while i < components.len() {
        // Try to start a group of consecutive gates of the same kind with matching key.
        let (kind, key) = match &components[i] {
            CW::Nand(n) => (RunKind::Nand, aligned_key(&n.a, &n.b, &n.out)),
            CW::And(n) => (RunKind::And, aligned_key(&n.a, &n.b, &n.out)),
            _ => (RunKind::Nand, None),
        };
        if let Some(k) = key {
            let start = i;
            i += 1;
            while i < components.len() {
                let next = match (&components[i], kind) {
                    (CW::Nand(n), RunKind::Nand) => aligned_key(&n.a, &n.b, &n.out),
                    (CW::And(n), RunKind::And) => aligned_key(&n.a, &n.b, &n.out),
                    _ => None,
                };
                if next == Some(k) {
                    i += 1;
                } else {
                    break;
                }
            }
            let run_len = i - start;
            if run_len >= 2 {
                let (a_id, b_id, out_id) = match &components[start] {
                    CW::Nand(n) => (n.a.id, n.b.id, n.out.id),
                    CW::And(n) => (n.a.id, n.b.id, n.out.id),
                    _ => unreachable!(),
                };
                // Nand(a, b) -> out
                result.push(CW::ParallelNand(wiring::ParallelNandWiring {
                    a: a_id,
                    b: b_id,
                    out: out_id,
                }));
                if kind == RunKind::And {
                    // Not(out) -> out  (invert in place to complete the And)
                    // Note: the "out" wire is being overwritten here, which isn't something that happens
                    // anywhere else, but should be fine.
                    result.push(CW::ParallelNand(wiring::ParallelNandWiring {
                        a: out_id,
                        b: out_id,
                        out: out_id,
                    }));
                }
            } else {
                result.push(components[start].clone());
            }
        } else {
            result.push(components[i].clone());
            i += 1;
        }
    }

    *components = result;
}

/// Coalesce trees of `AndWiring` gates that reduce multiple bits of the same source wire
/// to a single output bit into a single `ManyWayAndWiring`. Trees are discovered by walking
/// backward from each And gate through intermediate And gates until leaf inputs are reached.
/// If all leaves reference the same WireIndex, the tree is replaced.
fn coalesce_many_way_ands(components: &mut Vec<wiring::ComponentWiring>) {
    use std::collections::{HashMap, HashSet};
    use wiring::ComponentWiring as CW;

    // Index And gates by their output BitRef, so we can walk backward through the tree.
    let mut and_by_out: HashMap<wiring::BitRef, usize> = HashMap::new();
    for (i, comp) in components.iter().enumerate() {
        if let CW::And(a) = comp {
            and_by_out.insert(a.out, i);
        }
    }

    if and_by_out.is_empty() {
        return;
    }

    // For each And output, count how many distinct And gates consume it.
    // A node with fan-out > 1 among And gates can't be cleanly claimed by one tree.
    let mut and_consumers: HashMap<wiring::BitRef, HashSet<usize>> = HashMap::new();
    for (i, comp) in components.iter().enumerate() {
        if let CW::And(a) = comp {
            if and_by_out.contains_key(&a.a) {
                and_consumers.entry(a.a).or_default().insert(i);
            }
            if and_by_out.contains_key(&a.b) {
                and_consumers.entry(a.b).or_default().insert(i);
            }
        }
    }

    // Walk backward from a BitRef, collecting leaf bits and tree node indices.
    // Returns None if the tree mixes source wires or an intermediate has fan-out > 1.
    fn collect_tree(
        bit: wiring::BitRef,
        and_by_out: &HashMap<wiring::BitRef, usize>,
        and_consumers: &HashMap<wiring::BitRef, HashSet<usize>>,
        components: &[CW],
        tree_nodes: &mut Vec<usize>,
        leaves: &mut Vec<wiring::BitRef>,
    ) -> bool {
        if let Some(&idx) = and_by_out.get(&bit) {
            // This bit is produced by an And gate. Only follow if at most 1 distinct consumer.
            if and_consumers.get(&bit).map_or(0, |s| s.len()) > 1 {
                leaves.push(bit);
                return true;
            }
            let CW::And(a) = &components[idx] else {
                unreachable!()
            };
            let (a_bit, b_bit) = (a.a, a.b);
            tree_nodes.push(idx);
            collect_tree(
                a_bit,
                and_by_out,
                and_consumers,
                components,
                tree_nodes,
                leaves,
            ) && collect_tree(
                b_bit,
                and_by_out,
                and_consumers,
                components,
                tree_nodes,
                leaves,
            )
        } else {
            // Leaf: not produced by an And in our index.
            leaves.push(bit);
            true
        }
    }

    // Find roots: And gates whose output is NOT consumed by another And gate.
    let mut is_interior: HashSet<usize> = HashSet::new();
    for comp in components.iter() {
        if let CW::And(a) = comp {
            if let Some(&idx) = and_by_out.get(&a.a) {
                is_interior.insert(idx);
            }
            if let Some(&idx) = and_by_out.get(&a.b) {
                is_interior.insert(idx);
            }
        }
    }

    let mut claimed: Vec<bool> = vec![false; components.len()];
    let mut replacements: HashMap<usize, wiring::ManyWayAndWiring> = HashMap::new();

    for (i, comp) in components.iter().enumerate() {
        if claimed[i] || is_interior.contains(&i) {
            continue;
        }
        let CW::And(root) = comp else { continue };

        let mut tree_nodes = Vec::new();
        let mut leaves = Vec::new();
        tree_nodes.push(i);

        let a_bit = root.a;
        let b_bit = root.b;
        if !collect_tree(
            a_bit,
            &and_by_out,
            &and_consumers,
            components,
            &mut tree_nodes,
            &mut leaves,
        ) {
            continue;
        }
        if !collect_tree(
            b_bit,
            &and_by_out,
            &and_consumers,
            components,
            &mut tree_nodes,
            &mut leaves,
        ) {
            continue;
        }

        // All leaves must reference the same WireIndex.
        if leaves.is_empty() {
            continue;
        }
        let source_wire = leaves[0].id;
        if !leaves.iter().all(|l| l.id == source_wire) {
            continue;
        }

        // Need at least 3 source bits (2+ And gates) to be worth coalescing.
        if leaves.len() < 3 {
            continue;
        }

        // Build mask.
        let mut mask: u64 = 0;
        for leaf in &leaves {
            mask |= 1u64 << leaf.offset;
        }

        // Mark all tree nodes as claimed.
        for &idx in &tree_nodes {
            claimed[idx] = true;
        }

        replacements.insert(
            i,
            wiring::ManyWayAndWiring {
                a: source_wire,
                out: root.out,
                mask,
            },
        );
    }

    if replacements.is_empty() {
        return;
    }

    // Rebuild: skip claimed And gates, insert ManyWayAnd at root positions.
    let mut result: Vec<CW> = Vec::with_capacity(components.len());
    for i in 0..components.len() {
        if let Some(mwa) = replacements.remove(&i) {
            result.push(CW::ManyWayAnd(mwa));
        } else if !claimed[i] {
            result.push(components[i].clone());
        }
    }

    *components = result;
}

/// Coalesce `AdderWiring` entries that form a ripple-carry chain into a single
/// `RippleAdderWiring`. Chains are discovered by following carry→c links across the full
/// component list (not just consecutive entries), since unrelated ops may be interleaved.
/// Matched adders are pulled out of the list and replaced by a single RippleAdderWiring
/// at the position of the last adder in the chain.
///
/// When all `b` wires in a chain are zero-valued constants (present in `zero_wires`),
/// they're treated as equivalent even if they have different WireIndexes.
fn coalesce_ripple_adders(
    components: &mut Vec<wiring::ComponentWiring>,
    zero_wires: &std::collections::HashSet<wiring::WireIndex>,
) {
    use std::collections::HashMap;
    use wiring::ComponentWiring as CW;

    // Index adders by their carry-in (c) BitRef, so we can follow the chain.
    let mut by_carry_in: HashMap<wiring::BitRef, usize> = HashMap::new();
    for (i, comp) in components.iter().enumerate() {
        if let CW::Adder(a) = comp {
            by_carry_in.insert(a.c, i);
        }
    }

    // Find chain heads: adders whose c input is NOT the carry output of another adder.
    let mut is_chain_interior: Vec<bool> = vec![false; components.len()];
    for (i, comp) in components.iter().enumerate() {
        if let CW::Adder(a) = comp {
            if let Some(&next) = by_carry_in.get(&a.carry) {
                if next != i {
                    is_chain_interior[next] = true;
                }
            }
        }
    }

    // Walk chains from each head.
    let mut claimed: Vec<bool> = vec![false; components.len()];
    let mut replacements: HashMap<usize, wiring::RippleAdderWiring> = HashMap::new();

    for i in 0..components.len() {
        if claimed[i] || is_chain_interior[i] {
            continue;
        }
        let CW::Adder(first) = &components[i] else {
            continue;
        };

        // Verify aligned offsets for the head.
        if first.a.offset != first.sum.offset {
            continue;
        }

        let a_wire = first.a.id;
        let b_wire = first.b.id;
        let b_is_zero = zero_wires.contains(&b_wire);
        let sum_wire = first.sum.id;
        let base_offset = first.a.offset;

        // Follow the carry chain.
        let mut chain = vec![i];
        let mut prev_carry = first.carry;
        let mut step = 1u8;
        loop {
            let Some(&next) = by_carry_in.get(&prev_carry) else {
                break;
            };
            let CW::Adder(a) = &components[next] else {
                break;
            };
            let expected_offset = base_offset.wrapping_add(step);
            // b wires must match, OR all be zero-valued constants.
            let b_ok = a.b.id == b_wire || (b_is_zero && zero_wires.contains(&a.b.id));
            if !b_ok
                || a.a.id != a_wire
                || a.sum.id != sum_wire
                || a.a.offset != expected_offset
                || a.sum.offset != expected_offset
            {
                break;
            }
            chain.push(next);
            prev_carry = a.carry;
            step += 1;
        }

        if chain.len() < 2 {
            continue;
        }

        let last_idx = *chain.last().unwrap();
        for &idx in &chain {
            claimed[idx] = true;
        }
        replacements.insert(
            last_idx,
            wiring::RippleAdderWiring {
                carry_in: first.c,
                a: a_wire,
                b: b_wire,
                out: sum_wire,
                carry_out: prev_carry,
                offset: base_offset,
                width: chain.len() as u8,
            },
        );
    }

    if replacements.is_empty() {
        return;
    }

    // Rebuild: skip claimed adders, insert RippleAdder at the last position of each chain.
    let mut result: Vec<CW> = Vec::with_capacity(components.len());
    for i in 0..components.len() {
        if let Some(ripple) = replacements.remove(&i) {
            result.push(CW::RippleAdder(ripple));
        } else if !claimed[i] {
            result.push(components[i].clone());
        }
    }

    *components = result;
}

/// Move components that exclusively feed one branch of a mux into that mux's branch list.
/// Applied recursively: muxes moved into a branch get their own branches populated too.
fn populate_mux_branches(
    components: &mut Vec<wiring::ComponentWiring>,
    extra_consumers: &[wiring::WireIndex],
) {
    use std::collections::HashSet;
    use wiring::ComponentWiring as CW;

    // For each wire, which component indices consume it?
    let mut consumers: HashMap<wiring::WireIndex, HashSet<usize>> = HashMap::new();
    let mut add_consumer = |w: wiring::WireIndex, j: usize| {
        consumers.entry(w).or_default().insert(j);
    };
    for (j, comp) in components.iter().enumerate() {
        // TODO: user reflect()?
        match comp {
            CW::Nand(n) => {
                add_consumer(n.a.id, j);
                add_consumer(n.b.id, j);
            }
            CW::And(n) => {
                add_consumer(n.a.id, j);
                add_consumer(n.b.id, j);
            }
            CW::ParallelNand(n) => {
                add_consumer(n.a, j);
                add_consumer(n.b, j);
            }
            CW::RippleAdder(a) => {
                add_consumer(a.a, j);
                add_consumer(a.b, j);
                add_consumer(a.carry_in.id, j);
            }
            CW::ManyWayAnd(m) => {
                add_consumer(m.a, j);
            }
            CW::Mux(m) => {
                add_consumer(m.sel.id, j);
                add_consumer(m.a0, j);
                add_consumer(m.a1, j);
            }
            CW::Adder(a) => {
                add_consumer(a.a.id, j);
                add_consumer(a.b.id, j);
                add_consumer(a.c.id, j);
            }
            CW::Register(r) => {
                add_consumer(r.write.id, j);
                add_consumer(r.data_in, j);
            }
            CW::RAM(r) => {
                add_consumer(r.write.id, j);
                add_consumer(r.data_in, j);
                add_consumer(r.addr, j);
            }
            CW::ROM(r) => {
                add_consumer(r.addr, j);
            }
            CW::Serial(s) => {
                add_consumer(s.write.id, j);
                add_consumer(s.data_in, j);
            }
            CW::MemorySystem(m) => {
                add_consumer(m.write.id, j);
                add_consumer(m.data_in, j);
                add_consumer(m.addr, j);
            }
        }
    }
    // Extra consumers (chip outputs) use a sentinel index that will never be claimed.
    let sentinel = components.len();
    for &w in extra_consumers {
        consumers.entry(w).or_default().insert(sentinel);
    }

    // Build producer map: output wire → list of component indices.
    let mut producers: HashMap<wiring::WireIndex, Vec<usize>> = HashMap::new();
    for (j, comp) in components.iter().enumerate() {
        match comp {
            CW::Nand(n) => producers.entry(n.out.id).or_default().push(j),
            CW::And(n) => producers.entry(n.out.id).or_default().push(j),
            CW::ParallelNand(n) => producers.entry(n.out).or_default().push(j),
            CW::ManyWayAnd(m) => producers.entry(m.out.id).or_default().push(j),
            CW::RippleAdder(a) => {
                producers.entry(a.out).or_default().push(j);
                producers.entry(a.carry_out.id).or_default().push(j);
            }
            CW::Mux(m) => producers.entry(m.out).or_default().push(j),
            CW::Adder(a) => {
                producers.entry(a.sum.id).or_default().push(j);
                producers.entry(a.carry.id).or_default().push(j);
            }
            _ => {}
        }
    }

    // Helper: get input wires of a component.
    fn input_wires(comp: &CW) -> Vec<wiring::WireIndex> {
        // TODO: use reflect()?
        match comp {
            CW::Nand(n) => vec![n.a.id, n.b.id],
            CW::And(n) => vec![n.a.id, n.b.id],
            CW::ParallelNand(n) => vec![n.a, n.b],
            CW::RippleAdder(a) => vec![a.a, a.b, a.carry_in.id],
            CW::ManyWayAnd(m) => vec![m.a],
            CW::Mux(m) => vec![m.sel.id, m.a0, m.a1],
            CW::Adder(a) => vec![a.a.id, a.b.id, a.c.id],
            CW::Register(r) => vec![r.write.id, r.data_in],
            CW::RAM(r) => vec![r.write.id, r.data_in, r.addr],
            CW::ROM(r) => vec![r.addr],
            CW::Serial(s) => vec![s.write.id, s.data_in],
            CW::MemorySystem(m) => vec![m.write.id, m.data_in, m.addr],
        }
    }

    /// Collect components that exclusively feed a branch wire using fixed-point iteration.
    /// Handles internal fan-out: if a wire fans out to two nands that are both in the
    /// candidate set, their shared producer becomes eligible too.
    ///
    /// `mux_ok_wires` lists the wires the mux consumes where we consider the mux an
    /// acceptable "external" consumer (the branch wire itself and sel). Wires consumed
    /// by the mux on any other port (i.e., the other branch) must NOT be claimed.
    fn collect_branch(
        wire: wiring::WireIndex,
        mux_idx: usize,
        mux_ok_wires: &HashSet<wiring::WireIndex>,
        consumers: &HashMap<wiring::WireIndex, HashSet<usize>>,
        producers: &HashMap<wiring::WireIndex, Vec<usize>>,
        components: &[CW],
        claimed: &[bool],
    ) -> Vec<usize> {
        let mut candidates: HashSet<usize> = HashSet::new();

        // Seed: producers of the branch wire, if the wire is exclusively consumed
        // by the mux (and nothing else outside).
        let wire_consumers = consumers.get(&wire).map_or(0, |s| s.len());
        if wire_consumers != 1 {
            return Vec::new();
        }

        // Add initial producers.
        if let Some(prods) = producers.get(&wire) {
            for &j in prods {
                if !claimed[j] {
                    candidates.insert(j);
                }
            }
        }

        // Fixed-point: keep expanding until no new candidates are found.
        loop {
            let mut grew = false;
            // Collect all input wires of current candidates.
            let input_wires_to_check: Vec<wiring::WireIndex> = candidates
                .iter()
                .flat_map(|&j| input_wires(&components[j]))
                .collect();

            for w in input_wires_to_check {
                let Some(wire_consumers) = consumers.get(&w) else {
                    continue;
                };
                // All consumers of this wire must be in candidates, or be the mux
                // consuming on an "ok" port (this branch or sel — NOT the other branch).
                let all_accounted = wire_consumers.iter().all(|&c| {
                    if candidates.contains(&c) {
                        return true;
                    }
                    if c == mux_idx {
                        return mux_ok_wires.contains(&w);
                    }
                    false
                });
                if !all_accounted {
                    continue;
                }
                // Add producers of this wire.
                if let Some(prods) = producers.get(&w) {
                    for &j in prods {
                        if !claimed[j] && candidates.insert(j) {
                            grew = true;
                        }
                    }
                }
            }
            if !grew {
                break;
            }
        }

        if candidates.is_empty() {
            return Vec::new();
        }

        // Topological sort: emit in evaluation order (dependencies before dependents).
        let mut sorted = Vec::with_capacity(candidates.len());
        let mut emitted: HashSet<usize> = HashSet::new();
        fn topo_visit(
            j: usize,
            candidates: &HashSet<usize>,
            producers: &HashMap<wiring::WireIndex, Vec<usize>>,
            components: &[CW],
            emitted: &mut HashSet<usize>,
            sorted: &mut Vec<usize>,
        ) {
            if !emitted.insert(j) {
                return;
            }
            // Visit dependencies first.
            let inputs = input_wires(&components[j]);
            for w in inputs {
                if let Some(prods) = producers.get(&w) {
                    for &p in prods {
                        if candidates.contains(&p) {
                            topo_visit(p, candidates, producers, components, emitted, sorted);
                        }
                    }
                }
            }
            sorted.push(j);
        }
        for &j in &candidates {
            topo_visit(
                j,
                &candidates,
                producers,
                components,
                &mut emitted,
                &mut sorted,
            );
        }
        sorted
    }

    // Recursively collect branch assignments for a mux and all nested muxes.
    // Pushes inner assignments BEFORE outer ones so that during assembly,
    // inner muxes get their branches populated first.
    fn collect_mux_branches(
        mux_idx: usize,
        consumers: &HashMap<wiring::WireIndex, HashSet<usize>>,
        producers: &HashMap<wiring::WireIndex, Vec<usize>>,
        components: &[CW],
        claimed: &mut Vec<bool>,
        assignments: &mut Vec<(usize, Vec<usize>, Vec<usize>)>,
    ) {
        let m = match &components[mux_idx] {
            CW::Mux(m) => m,
            _ => return,
        };
        let (a0, a1) = (m.a0, m.a1);
        // For branch0: the mux consuming a wire on a0 is ok; on a1 or sel is NOT ok.
        // sel producers must remain top-level — they're needed before the branch decision.
        let ok_for_b0: HashSet<wiring::WireIndex> = [a0].into();
        let ok_for_b1: HashSet<wiring::WireIndex> = [a1].into();
        let b0 = collect_branch(
            a0, mux_idx, &ok_for_b0, consumers, producers, components, claimed,
        );
        let b1 = collect_branch(
            a1, mux_idx, &ok_for_b1, consumers, producers, components, claimed,
        );

        // Mark collected components as claimed.
        for &j in b0.iter().chain(b1.iter()) {
            claimed[j] = true;
        }

        // Recurse into any muxes we just claimed — BEFORE pushing our own assignment.
        for &j in b0.iter().chain(b1.iter()) {
            if matches!(&components[j], CW::Mux(_)) {
                collect_mux_branches(j, consumers, producers, components, claimed, assignments);
            }
        }

        if !b0.is_empty() || !b1.is_empty() {
            assignments.push((mux_idx, b0, b1));
        }
    }

    // For each top-level mux, collect exclusive producers into branches.
    // Process in reverse order so outermost muxes claim inner ones first.
    let mut claimed: Vec<bool> = vec![false; components.len()];
    let mut assignments: Vec<(usize, Vec<usize>, Vec<usize>)> = Vec::new();

    // Track which muxes are top-level branch owners (not to be extracted).
    let mut branch_roots: HashSet<usize> = HashSet::new();

    for i in (0..components.len()).rev() {
        if claimed[i] {
            continue;
        }
        if matches!(&components[i], CW::Mux(_)) {
            collect_mux_branches(
                i,
                &consumers,
                &producers,
                components,
                &mut claimed,
                &mut assignments,
            );
            branch_roots.insert(i);
            // The mux was marked claimed inside collect_mux_branches so its a0/a1
            // consumers count as claimed. But it stays in the top-level list.
        }
    }

    if assignments.is_empty() {
        return;
    }

    // Extract claimed components EXCEPT top-level branch roots.

    let mut extracted: Vec<Option<CW>> = vec![None; components.len()];
    for j in (0..components.len()).rev() {
        if claimed[j] && !branch_roots.contains(&j) {
            extracted[j] = Some(components.remove(j));
        }
    }

    // Compute new positions for non-extracted components.
    let extracted_set: HashSet<usize> = (0..claimed.len())
        .filter(|&j| claimed[j] && !branch_roots.contains(&j))
        .collect();
    let mut shift_at: Vec<usize> = vec![0; claimed.len() + 1];
    for j in 0..claimed.len() {
        shift_at[j + 1] = shift_at[j] + if extracted_set.contains(&j) { 1 } else { 0 };
    }

    // Assemble branches. Assignments are ordered inner-first, so when an outer mux
    // clones an inner mux from `extracted`, the inner mux already has its branches set.
    for (orig_idx, b0_indices, b1_indices) in &assignments {
        let b0: Vec<CW> = b0_indices
            .iter()
            .map(|&j| extracted[j].clone().unwrap())
            .collect();
        let b1: Vec<CW> = b1_indices
            .iter()
            .map(|&j| extracted[j].clone().unwrap())
            .collect();

        if branch_roots.contains(orig_idx) {
            // Top-level mux — update it in the components list
            let new_idx = orig_idx - shift_at[*orig_idx];
            if let CW::Mux(m) = &mut components[new_idx] {
                m.branch0 = b0;
                m.branch1 = b1;
            }
        } else {
            // Nested mux — update it in-place in extracted
            if let Some(Some(CW::Mux(m))) = extracted.get_mut(*orig_idx) {
                m.branch0 = b0;
                m.branch1 = b1;
            }
        }
    }
}

/// Topological sort of component_wiring so that dependencies are evaluated before dependents.
///
/// Only logic gates participate in the sort. Stateful components (registers, RAM, ROM, etc.)
/// have their outputs seeded externally, so they are excluded from the producer map and
/// appended at the end in their original order.
fn topo_sort_wiring(components: &mut Vec<wiring::ComponentWiring>) {
    use std::collections::HashMap;
    use wiring::ComponentWiring as CW;

    fn is_logic(comp: &CW) -> bool {
        matches!(
            comp,
            CW::Nand(_)
                | CW::And(_)
                | CW::ParallelNand(_)
                | CW::RippleAdder(_)
                | CW::ManyWayAnd(_)
                | CW::Mux(_)
                | CW::Adder(_)
        )
    }

    fn output_wires(comp: &CW) -> Vec<wiring::WireIndex> {
        match comp {
            CW::Nand(n) => vec![n.out.id],
            CW::And(n) => vec![n.out.id],
            CW::ParallelNand(n) => vec![n.out],
            CW::RippleAdder(a) => vec![a.out],
            CW::ManyWayAnd(m) => vec![m.out.id],
            CW::Mux(m) => vec![m.out],
            CW::Adder(a) => vec![a.sum.id, a.carry.id],
            _ => vec![],
        }
    }

    fn input_wires(comp: &CW) -> Vec<wiring::WireIndex> {
        match comp {
            CW::Nand(n) => vec![n.a.id, n.b.id],
            CW::And(n) => vec![n.a.id, n.b.id],
            CW::ParallelNand(n) => vec![n.a, n.b],
            CW::RippleAdder(a) => vec![a.a, a.b, a.carry_in.id],
            CW::ManyWayAnd(m) => vec![m.a],
            CW::Mux(m) => vec![m.sel.id, m.a0, m.a1],
            CW::Adder(a) => vec![a.a.id, a.b.id, a.c.id],
            _ => vec![],
        }
    }

    // Map output wire → producer index (logic gates only).
    let mut producers: HashMap<wiring::WireIndex, usize> = HashMap::new();
    for (i, comp) in components.iter().enumerate() {
        if is_logic(comp) {
            for w in output_wires(comp) {
                producers.insert(w, i);
            }
        }
    }

    let n = components.len();
    let mut visited = vec![false; n];
    let mut order = Vec::with_capacity(n);

    fn visit(
        i: usize,
        components: &[CW],
        producers: &HashMap<wiring::WireIndex, usize>,
        visited: &mut [bool],
        order: &mut Vec<usize>,
    ) {
        if visited[i] {
            return;
        }
        visited[i] = true;
        for w in input_wires(&components[i]) {
            if let Some(&dep) = producers.get(&w) {
                visit(dep, components, producers, visited, order);
            }
        }
        order.push(i);
    }

    // Sort logic gates first.
    for i in 0..n {
        if is_logic(&components[i]) {
            visit(i, components, &producers, &mut visited, &mut order);
        }
    }
    // Append stateful components in their original order.
    for i in 0..n {
        if !is_logic(&components[i]) {
            order.push(i);
        }
    }

    let mut sorted: Vec<CW> = Vec::with_capacity(n);
    for &i in &order {
        sorted.push(components[i].clone());
    }
    *components = sorted;
}
