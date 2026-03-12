use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use crate::declare::{BusRef, IC, Reflect as _};
use crate::component::{Computational, Computational16};
use crate::device::MemoryDevice as _;

type DeviceRAM = Rc<RefCell<crate::device::RAM>>;
type MSDevice   = crate::device::MemorySystem<DeviceRAM>;
type Indexes    = HashMap<WireID, WireIndex>;

/// Transform a circuit description into a pre-computed wiring layout.
///
/// No RAM or ROM buffers are allocated here. Call [`initialize`] to create a runnable
/// [`ChipState`].
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

/// Allocate simulation state (RAM/ROM buffers, registers) and run an initial evaluation.
pub fn initialize(wiring: ChipWiring) -> ChipState {
    let n_wires = wiring.n_wires;

    let ram_devices: Vec<DeviceRAM> = wiring.ram_specs.iter()
        .map(|s| Rc::new(RefCell::new(crate::device::RAM::new(s.size))))
        .collect();

    let rom_devices: Vec<Rc<RefCell<crate::device::ROM>>> = wiring.rom_specs.iter()
        .map(|s| Rc::new(RefCell::new(crate::device::ROM::new(s.size))))
        .collect();

    let mut ms_region_handles: Vec<RAMHandle> = Vec::new();
    let ms_devices: Vec<Rc<RefCell<MSDevice>>> = wiring.ms_specs.iter().map(|spec| {
        let mut overlays: Vec<crate::device::Overlay<DeviceRAM>> = Vec::new();
        for region in &spec.regions {
            let ram: DeviceRAM = Rc::new(RefCell::new(crate::device::RAM::new(region.size)));
            ms_region_handles.push(RAMHandle { base: region.base, inner: Rc::clone(&ram) });
            overlays.push(crate::device::Overlay { base: region.base, device: ram });
        }
        Rc::new(RefCell::new(MSDevice { devices: overlays }))
    }).collect();

    let mut bus_residents: Vec<BusResident> = Vec::new();
    for ram in &ram_devices {
        bus_residents.push(BusResident::RAM(RAMHandle { base: 0, inner: Rc::clone(ram) }));
    }
    for rom in &rom_devices {
        bus_residents.push(BusResident::ROM(ROMHandle { inner: Rc::clone(rom) }));
    }
    bus_residents.extend(ms_region_handles.into_iter().map(BusResident::RAM));

    let mut state = ChipState {
        wiring,
        ram_devices,
        rom_devices,
        ms_devices,
        bus_residents,
        reg_state:  vec![0u64; n_wires],
        input_vals: HashMap::new(),
        dirty: false,
        wire_state: vec![0u64; n_wires],
    };
    state.evaluate();
    state
}

/// Static, synthesized description of the circuit's wiring. Computed once and never mutated.
pub struct ChipWiring {
    component_wiring: Vec<wiring::ComponentWiring>,
    input_wiring:  HashMap<String, wiring::WireRef>,
    output_wiring: HashMap<String, wiring::WireRef>,
    n_wires: usize,
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

/// Runtime state of a simulated chip, and access to its inputs and outputs.
pub struct ChipState {
    wiring:       ChipWiring,
    ram_devices:  Vec<DeviceRAM>,
    rom_devices:  Vec<Rc<RefCell<crate::device::ROM>>>,
    ms_devices:   Vec<Rc<RefCell<MSDevice>>>,
    bus_residents: Vec<BusResident>,
    reg_state:    Vec<u64>,
    input_vals:   HashMap<wiring::WireRef, u64>,
    dirty:        bool,
    wire_state:   Vec<u64>,
}

/// Arbitrary (ptr) value which identifies the storage location for some wire, used as a key to
/// store states in a HashMap, so it only needs to be unique to each wire and hashable.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct WireID(usize);

impl From<&BusRef> for WireID {
    fn from(busref: &BusRef) -> Self {
        WireID(Rc::as_ptr(&busref.id) as usize)
    }
}

/// Index of the storage location of a wire within a flat buffer. Each wire has a unique index,
/// running from 0 up to the total number of distinct wires in the circuit.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct WireIndex(u32);

/// Pre-computed wiring info about components, used during evaluation.
mod wiring {
    use crate::component::{Nand, Register16, RAM16, ROM16, MemorySystem16};
    use crate::declare::{BusRef, Reflect};
    use super::{WireIndex, WireID, Indexes};

    pub(super) enum ComponentWiring {
        Nand(NandWiring),
        Register(RegisterWiring),
        ROM(ROMWiring),
        RAM(RAMWiring),
        MemorySystem(MemorySystemWiring),
        /// Note: output wiring for consts is not needed during evaluation because the bits are
        /// never updated.
        Const,
    }

    #[derive(Clone, Copy, PartialEq, Eq, Hash)]
    pub(super) struct BitRef { pub(super) id: WireIndex, pub(super) offset: u8 }
    impl BitRef {
        pub(super) fn new(b: &BusRef, ix: &Indexes) -> Self { BitRef { id: ix[&WireID::from(b)], offset: b.offset as u8 } }
    }

    #[derive(Clone, Copy, PartialEq, Eq, Hash)]
    pub(super) struct WireRef { pub(super) id: WireIndex, pub(super) offset: u8, pub(super) width: u8 }
    impl WireRef {
        pub(super) fn new(b: &BusRef, ix: &Indexes) -> Self { WireRef { id: ix[&WireID::from(b)], offset: b.offset as u8, width: b.width as u8 } }
    }

    pub(super) struct NandWiring { pub(super) a: BitRef, pub(super) b: BitRef, pub(super) out: BitRef }
    impl NandWiring {
        pub(super) fn new(nand: &Nand, ix: &Indexes) -> Self {
            let intf = nand.reflect();
            Self {
                a:   BitRef::new(&intf.inputs["a"], ix),
                b:   BitRef::new(&intf.inputs["b"], ix),
                out: BitRef::new(&intf.outputs["out"], ix),
            }
        }
    }

    pub(super) struct RegisterWiring { pub(super) write: BitRef, pub(super) data_in: WireRef, pub(super) data_out: WireIndex }
    impl RegisterWiring {
        pub(super) fn new(reg: &Register16, ix: &Indexes) -> Self {
            let intf = reg.reflect();
            Self {
                write:    BitRef::new(&intf.inputs["write"], ix),
                data_in:  WireRef::new(&intf.inputs["data_in"], ix),
                data_out: ix[&WireID::from(&intf.outputs["data_out"])],
            }
        }
    }

    pub(super) struct ROMWiring { pub(super) device_slot: usize, pub(super) out: WireRef, pub(super) addr: WireRef }
    impl ROMWiring {
        pub(super) fn new(rom: &ROM16, slot: usize, ix: &Indexes) -> Self {
            let intf = rom.reflect();
            Self {
                device_slot: slot,
                out:  WireRef::new(&intf.outputs["out"], ix),
                addr: WireRef::new(&intf.inputs["addr"], ix),
            }
        }
    }

    pub(super) struct RAMWiring { pub(super) device_slot: usize, pub(super) out: WireRef, pub(super) addr: WireRef, pub(super) write: BitRef, pub(super) data_in: WireRef }
    impl RAMWiring {
        pub(super) fn new(ram: &RAM16, slot: usize, ix: &Indexes) -> Self {
            let intf = ram.reflect();
            Self {
                device_slot: slot,
                out:     WireRef::new(&intf.outputs["data_out"], ix),
                addr:    WireRef::new(&intf.inputs["addr"], ix),
                write:   BitRef::new(&intf.inputs["write"], ix),
                data_in: WireRef::new(&intf.inputs["data_in"], ix),
            }
        }
    }

    pub(super) struct MemorySystemWiring { pub(super) device_slot: usize, pub(super) out: WireRef, pub(super) addr: WireRef, pub(super) write: BitRef, pub(super) data_in: WireRef }
    impl MemorySystemWiring {
        pub(super) fn new(ms: &MemorySystem16, slot: usize, ix: &Indexes) -> Self {
            let intf = ms.reflect();
            Self {
                device_slot: slot,
                out:     WireRef::new(&intf.outputs["data_out"], ix),
                addr:    WireRef::new(&intf.inputs["addr"], ix),
                write:   BitRef::new(&intf.inputs["write"], ix),
                data_in: WireRef::new(&intf.inputs["data_in"], ix),
            }
        }
    }
}

impl ChipState {

    /// Set the value of an input. Combinational outputs will reflect this on the next `get()`.
    pub fn set(&mut self, name: &str, value: u64) {
        if let Some(&wr) = self.wiring.input_wiring.get(name) {
            self.input_vals.insert(wr, value);
        }
        self.dirty = true;
    }

    /// Get the value of an output, re-evaluating combinational logic if any inputs changed.
    pub fn get(&mut self, name: &str) -> u64 {
        if self.dirty {
            self.evaluate();
            self.dirty = false;
        }
        self.wiring.output_wiring.get(name)
            .map(|&wr| read_bus(&self.wire_state, wr))
            .unwrap_or(0)
    }

    /// RAM and ROM instances present in the simulated circuit.
    pub fn bus_residents(&self) -> &[BusResident] {
        &self.bus_residents
    }

    /// RAM and ROM instances present in the simulated circuit, mutably (e.g. to load a ROM).
    pub fn bus_residents_mut(&mut self) -> &mut [BusResident] {
        &mut self.bus_residents
    }

    /// Turn the crank: latch registers and RAM, then re-evaluate combinational logic.
    pub fn ticktock(&mut self) {
        // Evaluate with current inputs so wire_state reflects this cycle.
        self.dirty = false;
        self.evaluate();

        for comp in &self.wiring.component_wiring {
            match comp {
                wiring::ComponentWiring::Register(reg) => {
                    if read_bit(&self.wire_state, reg.write) {
                        let val = read_bus(&self.wire_state, reg.data_in);
                        self.reg_state[reg.data_out.0 as usize] = val;
                    }
                }
                wiring::ComponentWiring::RAM(ram) => {
                    if read_bit(&self.wire_state, ram.write) {
                        let val = read_bus(&self.wire_state, ram.data_in);
                        let _ = self.ram_devices[ram.device_slot].borrow_mut().write(val);
                    }
                }
                wiring::ComponentWiring::MemorySystem(ms) => {
                    if read_bit(&self.wire_state, ms.write) {
                        let val = read_bus(&self.wire_state, ms.data_in);
                        let _ = self.ms_devices[ms.device_slot].borrow_mut().write(val);
                    }
                }
                _ => {}
            }
        }

        // Latch RAM and MS addr from the initial wire_state so the re-evaluate below
        // shows the correct memory data.
        for comp in &self.wiring.component_wiring {
            match comp {
                wiring::ComponentWiring::RAM(ram) => {
                    let new_addr = read_bus(&self.wire_state, ram.addr);
                    let _ = self.ram_devices[ram.device_slot].borrow_mut().set_addr(new_addr as usize);
                    self.ram_devices[ram.device_slot].borrow_mut().ticktock();
                }
                wiring::ComponentWiring::MemorySystem(ms) => {
                    let new_addr = read_bus(&self.wire_state, ms.addr);
                    let _ = self.ms_devices[ms.device_slot].borrow_mut().set_addr(new_addr as usize);
                    self.ms_devices[ms.device_slot].borrow_mut().ticktock();
                }
                _ => {}
            }
        }

        // Re-evaluate with updated registers, writes, and new MS latched addr.
        self.evaluate();
        self.dirty = false;

        // Latch ROM addr after re-evaluate so the next cycle processes the *current*
        // instruction, which lets the CPU's feed-forward next_addr_mux set the right MS
        // addr latch for the cycle after.
        for comp in &self.wiring.component_wiring {
            if let wiring::ComponentWiring::ROM(rom) = comp {
                let new_addr = read_bus(&self.wire_state, rom.addr);
                let _ = self.rom_devices[rom.device_slot].borrow_mut().set_addr(new_addr as usize);
            }
        }
    }

    fn evaluate(&mut self) {
        // Start fresh: reg outputs are the base state.
        self.wire_state.copy_from_slice(&self.reg_state);

        // Seed chip inputs (may overwrite reg values on shared wires).
        for (&wr, &val) in &self.input_vals {
            write_bus(&mut self.wire_state, wr, val);
        }

        // Seed RAM/ROM/MS outputs from their current addr input.
        // The addr wire is either an external chip input (seeded above) or a register output
        // (seeded from reg_state above), so it's available in wire_state before the Nand passes.
        for comp in &self.wiring.component_wiring {
            match comp {
                wiring::ComponentWiring::RAM(ram) => {
                    let val = self.ram_devices[ram.device_slot].borrow().read().unwrap_or(0);
                    write_bus(&mut self.wire_state, ram.out, val);
                }
                wiring::ComponentWiring::ROM(rom) => {
                    let val = self.rom_devices[rom.device_slot].borrow().read().unwrap_or(0);
                    write_bus(&mut self.wire_state, rom.out, val);
                }
                wiring::ComponentWiring::MemorySystem(ms) => {
                    let val = self.ms_devices[ms.device_slot].borrow().read().unwrap_or(0);
                    write_bus(&mut self.wire_state, ms.out, val);
                }
                _ => {}
            }
        }

        // Two Nand passes: first propagates RAM/ROM outputs through memory logic
        // (e.g. MemorySystem muxes), second lets downstream gates (ALU) use the
        // correctly computed values. Needed because component order puts CPU before
        // MemorySystem in the flattened list.
        eval_nands(&mut self.wire_state, &self.wiring.component_wiring);
        eval_nands(&mut self.wire_state, &self.wiring.component_wiring);
    }
}

fn eval_nands(ws: &mut [u64], component_wiring: &[wiring::ComponentWiring]) {
    for comp in component_wiring {
        if let wiring::ComponentWiring::Nand(nand) = comp {
            let a = read_bit(ws, nand.a);
            let b = read_bit(ws, nand.b);
            write_bit(ws, nand.out, !(a & b));
        }
    }
}

fn width_mask(width: usize) -> u64 {
    if width >= 64 { u64::MAX } else { (1u64 << width) - 1 }
}

fn read_bus(ws: &[u64], b: wiring::WireRef) -> u64 {
    (ws[b.id.0 as usize] >> b.offset) & width_mask(b.width as usize)
}

fn write_bus(ws: &mut [u64], b: wiring::WireRef, value: u64) {
    let mask = width_mask(b.width as usize);
    ws[b.id.0 as usize] = (ws[b.id.0 as usize] & !(mask << b.offset)) | ((value & mask) << b.offset);
}

fn read_bit(ws: &[u64], b: wiring::BitRef) -> bool {
    (ws[b.id.0 as usize] >> b.offset) & 1 != 0
}

fn write_bit(ws: &mut [u64], b: wiring::BitRef, value: bool) {
    let bit = 1u64 << b.offset;
    if value { ws[b.id.0 as usize] |= bit; } else { ws[b.id.0 as usize] &= !bit; }
}

/// Access to auxiliary devices "on the bus" which the harness needs to inspect.
pub enum BusResident {
    RAM(RAMHandle),
    ROM(ROMHandle),
    // Future: Keyboard(KeyboardHandle),
    // Future: TTY(TTYHandle),
}

/// A clonable handle to a RAM instance (standalone or a region within a MemorySystem).
///
/// `base` is the region's base address in the memory map (0 for standalone RAM).
#[derive(Clone)]
pub struct RAMHandle {
    pub base: usize,
    inner: Rc<RefCell<crate::device::RAM>>,
}

impl RAMHandle {
    pub fn peek(&self, addr: u64) -> u64    { self.inner.borrow().peek(addr as usize).unwrap_or(0) }
    pub fn poke(&self, addr: u64, val: u64) { let _ = self.inner.borrow_mut().poke(addr as usize, val); }
    pub fn size(&self) -> usize             { self.inner.borrow().size }
}

/// A clonable handle to a ROM instance in the simulated circuit.
#[derive(Clone)]
pub struct ROMHandle {
    inner: Rc<RefCell<crate::device::ROM>>,
}

impl ROMHandle {
    pub fn flash(&self, data: Vec<u64>) {
        let _ = self.inner.borrow_mut().flash(data.into_boxed_slice());
    }
    pub fn size(&self) -> usize { self.inner.borrow().size }
}

/// Descriptor for one contiguous RAM region in a memory map.
pub struct RAMMap {
    pub size: usize,
    pub base: usize,
}

/// Descriptor for the memory layout passed to [`synthesize`].
///
/// Specifies which regions exist and where they appear in the address space.
/// All actual data storage lives in [`device::RAM`] instances created by [`initialize`].
pub struct MemoryMap {
    pub contents: Vec<RAMMap>,
}

impl MemoryMap {
    pub fn new(contents: Vec<RAMMap>) -> Self {
        MemoryMap { contents }
    }
}
