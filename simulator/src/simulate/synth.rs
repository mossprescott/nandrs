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
    /// One entry per Serial I/O component.
    pub serial_specs: Vec<SerialSpec>,
    /// One entry per MemorySystem component, including the RAM region layout.
    pub ms_specs: Vec<MemorySystemSpec>,
}

/// Descriptor for a standalone RAM component.
pub struct RAMSpec { pub size: usize }

/// Descriptor for a ROM component.
pub struct ROMSpec { pub size: usize }

/// Descriptor for a Serial I/O component.
pub struct SerialSpec;

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

fn fmt_component_tree(f: &mut fmt::Formatter<'_>, comp: &wiring::ComponentWiring, indent: &str) -> fmt::Result {
    match comp {
        wiring::ComponentWiring::Nand(n) if n.a == n.b =>
            writeln!(f, "not   a={} out={}",
                fmt_bit(n.a), fmt_bit(n.out)),
        wiring::ComponentWiring::Nand(n) =>
            writeln!(f, "nand  a={} b={} out={}",
                fmt_bit(n.a), fmt_bit(n.b), fmt_bit(n.out)),
        wiring::ComponentWiring::Mux(m) => {
            writeln!(f, "mux   sel={} out=w{}[..]",
                fmt_bit(m.sel), m.out.0)?;
            let (t0, _) = count_gates(&m.branch0);
            let (t1, _) = count_gates(&m.branch1);
            writeln!(f, "{indent}     a0=w{}[..] ({} gates)", m.a0.0, t0)?;
            let inner = format!("{indent}       ");
            for op in &m.branch0 {
                write!(f, "{inner}")?;
                fmt_component_tree(f, op, &inner)?;
            }
            writeln!(f, "{indent}     a1=w{}[..] ({} gates)", m.a1.0, t1)?;
            for op in &m.branch1 {
                write!(f, "{inner}")?;
                fmt_component_tree(f, op, &inner)?;
            }
            Ok(())
        }

        wiring::ComponentWiring::Register(r) =>
            writeln!(f, "reg   write={} in=w{}[..] out=w{}[..]",
                fmt_bit(r.write), r.data_in.0, r.data_out.0),

        wiring::ComponentWiring::RAM(r) =>
            writeln!(f, "ram[{}]  addr=w{}[..] write={} in=w{}[..] out=w{}[..]",
                r.device_slot, r.addr.0, fmt_bit(r.write), r.data_in.0, r.out.0),
        wiring::ComponentWiring::ROM(r) =>
            writeln!(f, "rom[{}]  addr=w{}[..] out=w{}[..]",
                r.device_slot, r.addr.0, r.out.0),
        wiring::ComponentWiring::Serial(s) =>
            writeln!(f, "serial[{}]  write={} in=w{}[..] out=w{}[..]",
                s.device_slot, fmt_bit(s.write), s.data_in.0, s.out.0),
        wiring::ComponentWiring::MemorySystem(m) =>
            writeln!(f, "mem[{}]  addr=w{}[..] write={} in=w{}[..] out=w{}[..]",
                m.device_slot, m.addr.0, fmt_bit(m.write), m.data_in.0, m.out.0),

        // synthetic:
        wiring::ComponentWiring::And(n) =>
            writeln!(f, "and   a={} b={} out={}",
                fmt_bit(n.a), fmt_bit(n.b), fmt_bit(n.out)),
    }
}

/// Count gates (nands + ands) in a list of components, recursing into mux branches.
/// Returns (total, min) where min assumes the cheaper branch is taken at each mux.
fn count_gates(components: &[wiring::ComponentWiring]) -> (u32, u32) {
    let mut total = 0u32;
    let mut min = 0u32;
    for comp in components {
        match comp {
            wiring::ComponentWiring::Nand(_) | wiring::ComponentWiring::And(_) => { total += 1; min += 1; }
            wiring::ComponentWiring::Mux(m) => {
                let (t0, m0) = count_gates(&m.branch0);
                let (t1, m1) = count_gates(&m.branch1);
                total += t0 + t1;
                min += m0.min(m1);
            }
            _ => {}
        }
    }
    (total, min)
}

impl fmt::Display for ChipWiring {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut muxes = 0u32;
        let mut registers = 0u32;
        for comp in &self.component_wiring {
            match comp {
                wiring::ComponentWiring::Mux(_)      => muxes += 1,
                wiring::ComponentWiring::Register(_) => registers += 1,
                _ => {}
            }
        }
        let (total_gates, min_gates) = count_gates(&self.component_wiring);
        writeln!(f, "ChipWiring:")?;
        write!(f, "  gates:     {} total", total_gates)?;
        if min_gates < total_gates {
            writeln!(f, ", {} min/cycle", min_gates)?;
        } else {
            writeln!(f)?;
        }
        if muxes > 0 { writeln!(f, "  muxes:     {}", muxes)?; }
        if registers > 0 { writeln!(f, "  registers: {}", registers)?; }
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
            for r in &ms.regions { writeln!(f, "    {} words @ 0x{:04x}", r.size, r.base)?; }
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
            writeln!(f, "  const: w{} = {}", cw.out.0, cw.value)?;
        }

        for (i, comp) in self.component_wiring.iter().enumerate() {
            write!(f, "  [{i}] ")?;
            fmt_component_tree(f, comp, "  ")?;
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
                Computational::Mux1(c) => {
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
                Computational::Serial(c) => {
                    let intf = c.reflect();
                    assign(WireID::from(&intf.outputs["data_out"]));
                    assign(WireID::from(&intf.inputs["write"]));
                    assign(WireID::from(&intf.inputs["data_in"]));
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
    let mut serial_specs: Vec<SerialSpec> = Vec::new();
    let mut ms_specs: Vec<MemorySystemSpec>  = Vec::new();

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
            Computational::Mux1(c)         => {
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
            Computational::Serial(_c) => {
                let slot = serial_specs.len();
                serial_specs.push(SerialSpec);

                let intf = _c.reflect();
                Some(CW::Serial(wiring::SerialWiring {
                    device_slot: slot,
                    out:     wire_indexes[&WireID::from(&intf.outputs["data_out"])],
                    write:   ref_for(&intf.inputs["write"]),
                    data_in: wire_indexes[&WireID::from(&intf.inputs["data_in"])],
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

    // Peephole: collapse nand+not into and.
    let mut component_wiring = component_wiring;
    let output_wires: Vec<wiring::WireIndex> = chip.reflect().outputs.values()
        .map(|b| wire_indexes[&WireID::from(b)])
        .collect();
    peephole_nand_not(&mut component_wiring, &output_wires);

    // Remove gates whose output is never consumed.
    eliminate_dead_gates(&mut component_wiring, &output_wires);

    // Post-processing: move exclusive producers into mux branches (recursively).
    populate_mux_branches(&mut component_wiring, &output_wires);

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
        serial_specs,
        ms_specs,
    }
}

/// Peephole: replace `nand a,b -> n; nand n,n -> out` with `and a,b -> out`.
/// The not (self-nand) must be the sole consumer of the nand's output.
fn peephole_nand_not(components: &mut Vec<wiring::ComponentWiring>, output_wires: &[wiring::WireIndex]) {
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
                wire_consumers.entry((n.a.id.0, n.a.offset)).or_default().insert(i);
                wire_consumers.entry((n.b.id.0, n.b.offset)).or_default().insert(i);
            }
            CW::And(n) => {
                wire_consumers.entry((n.a.id.0, n.a.offset)).or_default().insert(i);
                wire_consumers.entry((n.b.id.0, n.b.offset)).or_default().insert(i);
            }
            CW::Mux(m) => {
                wire_consumers.entry((m.sel.id.0, m.sel.offset)).or_default().insert(i);
                bus_consumed.insert(m.a0.0);
                bus_consumed.insert(m.a1.0);
            }
            CW::Register(r) => {
                wire_consumers.entry((r.write.id.0, r.write.offset)).or_default().insert(i);
                bus_consumed.insert(r.data_in.0);
            }
            CW::RAM(r) => {
                wire_consumers.entry((r.write.id.0, r.write.offset)).or_default().insert(i);
                bus_consumed.insert(r.data_in.0);
                bus_consumed.insert(r.addr.0);
            }
            CW::ROM(r) => {
                bus_consumed.insert(r.addr.0);
            }
            CW::Serial(s) => {
                wire_consumers.entry((s.write.id.0, s.write.offset)).or_default().insert(i);
                bus_consumed.insert(s.data_in.0);
            }
            CW::MemorySystem(m) => {
                wire_consumers.entry((m.write.id.0, m.write.offset)).or_default().insert(i);
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
            if bus_consumed.contains(&n.out.id.0) { continue; }
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
        if to_remove.contains(&i) { continue; }
        let CW::Nand(not_gate) = &components[i] else { continue };
        if not_gate.a != not_gate.b { continue; }
        let key = (not_gate.a.id.0, not_gate.a.offset);
        let Some(&nand_idx) = nand_by_out.get(&key) else { continue };
        if nand_idx == i { continue; }
        if to_remove.contains(&nand_idx) { continue; }
        // nand_idx may have already been converted to And by a prior iteration.
        let CW::Nand(nand_gate) = &components[nand_idx] else { continue };
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
fn eliminate_dead_gates(components: &mut Vec<wiring::ComponentWiring>, output_wires: &[wiring::WireIndex]) {
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
                CW::Mux(m) => {
                    consumed_bits.insert((m.sel.id.0, m.sel.offset));
                    bus_consumed.insert(m.a0.0);
                    bus_consumed.insert(m.a1.0);
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
                _ => true, // keep registers, RAM, ROM, muxes, memory systems
            }
        });

        if components.len() == before { break; }
    }
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
    let mut add_consumer = |w: wiring::WireIndex, j: usize| { consumers.entry(w).or_default().insert(j); };
    for (j, comp) in components.iter().enumerate() {
        match comp {
            CW::Nand(n)         => { add_consumer(n.a.id, j); add_consumer(n.b.id, j); }
            CW::And(n)          => { add_consumer(n.a.id, j); add_consumer(n.b.id, j); }
            CW::Mux(m)          => { add_consumer(m.sel.id, j); add_consumer(m.a0, j); add_consumer(m.a1, j); }
            CW::Register(r)     => { add_consumer(r.write.id, j); add_consumer(r.data_in, j); }
            CW::RAM(r)          => { add_consumer(r.write.id, j); add_consumer(r.data_in, j); add_consumer(r.addr, j); }
            CW::ROM(r)          => { add_consumer(r.addr, j); }
            CW::Serial(s)       => { add_consumer(s.write.id, j); add_consumer(s.data_in, j); }
            CW::MemorySystem(m) => { add_consumer(m.write.id, j); add_consumer(m.data_in, j); add_consumer(m.addr, j); }
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
            CW::And(n)  => producers.entry(n.out.id).or_default().push(j),
            CW::Mux(m)  => producers.entry(m.out).or_default().push(j),
            _ => {}
        }
    }

    // Helper: get input wires of a component.
    fn input_wires(comp: &CW) -> Vec<wiring::WireIndex> {
        match comp {
            CW::Nand(n)         => vec![n.a.id, n.b.id],
            CW::And(n)          => vec![n.a.id, n.b.id],
            CW::Mux(m)          => vec![m.sel.id, m.a0, m.a1],
            CW::Register(r)     => vec![r.write.id, r.data_in],
            CW::RAM(r)          => vec![r.write.id, r.data_in, r.addr],
            CW::ROM(r)          => vec![r.addr],
            CW::Serial(s)       => vec![s.write.id, s.data_in],
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
                if !claimed[j] { candidates.insert(j); }
            }
        }

        // Fixed-point: keep expanding until no new candidates are found.
        loop {
            let mut grew = false;
            // Collect all input wires of current candidates.
            let input_wires_to_check: Vec<wiring::WireIndex> = candidates.iter()
                .flat_map(|&j| input_wires(&components[j]))
                .collect();

            for w in input_wires_to_check {
                let Some(wire_consumers) = consumers.get(&w) else { continue };
                // All consumers of this wire must be in candidates, or be the mux
                // consuming on an "ok" port (this branch or sel — NOT the other branch).
                let all_accounted = wire_consumers.iter().all(|&c| {
                    if candidates.contains(&c) { return true; }
                    if c == mux_idx { return mux_ok_wires.contains(&w); }
                    false
                });
                if !all_accounted { continue; }
                // Add producers of this wire.
                if let Some(prods) = producers.get(&w) {
                    for &j in prods {
                        if !claimed[j] && candidates.insert(j) {
                            grew = true;
                        }
                    }
                }
            }
            if !grew { break; }
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
            if !emitted.insert(j) { return; }
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
            topo_visit(j, &candidates, producers, components, &mut emitted, &mut sorted);
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
        let m = match &components[mux_idx] { CW::Mux(m) => m, _ => return };
        let (a0, a1) = (m.a0, m.a1);
        // For branch0: the mux consuming a wire on a0 is ok; on a1 or sel is NOT ok.
        // sel producers must remain top-level — they're needed before the branch decision.
        let ok_for_b0: HashSet<wiring::WireIndex> = [a0].into();
        let ok_for_b1: HashSet<wiring::WireIndex> = [a1].into();
        let b0 = collect_branch(a0, mux_idx, &ok_for_b0, consumers, producers, components, claimed);
        let b1 = collect_branch(a1, mux_idx, &ok_for_b1, consumers, producers, components, claimed);

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
        if claimed[i] { continue; }
        if matches!(&components[i], CW::Mux(_)) {
            collect_mux_branches(i, &consumers, &producers, components, &mut claimed, &mut assignments);
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
        let b0: Vec<CW> = b0_indices.iter().map(|&j| extracted[j].clone().unwrap()).collect();
        let b1: Vec<CW> = b1_indices.iter().map(|&j| extracted[j].clone().unwrap()).collect();

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
