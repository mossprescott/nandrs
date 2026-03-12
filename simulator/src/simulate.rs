use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use crate::declare::{BusRef, IC, Interface, Reflect as _};
use crate::component::{Computational, Computational16};
use crate::device::MemoryDevice as _;

type DeviceRAM = Rc<RefCell<crate::device::RAM>>;
type MSDevice   = crate::device::MemorySystem<DeviceRAM>;

/// Transform circuit description for simulation.
///
/// Note: currently 16-bit words are assumed, but up to 64-bits wouldn't be a problem if the type
/// was generalized.
pub fn synthesize<C>(chip: &IC<C>, memory_map: MemoryMap) -> ChipState
where
    C: Clone + crate::Reflect + Into<Computational16>,
{
    let components: Vec<Computational16> = chip.components.iter().cloned().map(Into::into).collect();
    let mut reg_state: HashMap<usize, u64> = HashMap::new();
    let mut bus_residents: Vec<BusResident> = Vec::new();
    let mut ms_handles: Vec<MSHandle> = Vec::new();
    let mut memory_map = Some(memory_map);
    for comp in &components {
        match comp {
            Computational::Register(reg) => {
                let intf = reg.reflect();
                assert_eq!(intf.outputs["data_out"].width, 16);
                reg_state.insert(wire_id(&intf.outputs["data_out"]), 0);
            }
            Computational::RAM(ram) => {
                let intf = ram.reflect();
                assert_eq!(intf.outputs["data_out"].width, 16);
                let out_id = wire_id(&intf.outputs["data_out"]);
                let inner: DeviceRAM = Rc::new(RefCell::new(crate::device::RAM::new(ram.size)));
                bus_residents.push(BusResident::RAM(RAMHandle { wire_id: out_id, base: 0, inner }));
            }
            Computational::ROM(rom) => {
                let intf = rom.reflect();
                bus_residents.push(BusResident::ROM(ROMHandle {
                    wire_id: wire_id(&intf.outputs["out"]),
                    inner: Rc::new(RefCell::new(crate::device::ROM::new(rom.size))),
                }));
            }
            Computational::MemorySystem(ms) => {
                let intf = ms.reflect();
                let out_id = wire_id(&intf.outputs["data_out"]);
                let map = memory_map.take().expect("only one MemorySystem supported");
                let mut overlays: Vec<crate::device::Overlay<DeviceRAM>> = Vec::new();
                for r in map.contents {
                    let ram: DeviceRAM = Rc::new(RefCell::new(crate::device::RAM::new(r.size)));
                    // wire_id=0: MS-region RAMs have no direct component wire; base identifies the region.
                    bus_residents.push(BusResident::RAM(RAMHandle { wire_id: 0, base: r.base, inner: Rc::clone(&ram) }));
                    overlays.push(crate::device::Overlay { base: r.base, device: ram });
                }
                ms_handles.push(MSHandle {
                    wire_id: out_id,
                    device: Rc::new(RefCell::new(MSDevice { devices: overlays })),
                });
            }
            _ => {}
        }
    }
    let mut state = ChipState {
        intf: chip.reflect(),
        components,
        input_vals: HashMap::new(),
        wire_state: HashMap::new(),
        reg_state,
        bus_residents,
        ms_handles,
        dirty: false,
    };
    state.evaluate();
    state
}

/// Runtime state of a simulated chip, and access to its inputs and outputs.
pub struct ChipState {
    intf: Interface,
    components: Vec<Computational16>,
    input_vals: HashMap<String, u64>,
    wire_state: HashMap<usize, u64>,
    reg_state: HashMap<usize, u64>,
    bus_residents: Vec<BusResident>,
    ms_handles: Vec<MSHandle>,
    dirty: bool,
}

impl ChipState {
    /// Set the value of an input. Combinational outputs will reflect this on the next `get()`.
    pub fn set(&mut self, name: &str, value: u64) {
        self.input_vals.insert(name.to_string(), value);
        self.dirty = true;
    }

    /// Get the value of an output, re-evaluating combinational logic if any inputs changed.
    pub fn get(&mut self, name: &str) -> u64 {
        if self.dirty {
            self.evaluate();
            self.dirty = false;
        }
        self.intf.outputs.get(name)
            .map(|b| read_bus(&self.wire_state, b))
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

        // Collect updates based on the current wire_state.
        let mut reg_updates: Vec<(usize, u64)> = Vec::new();
        let mut ram_writes: Vec<(usize, u64, u64)> = Vec::new();  // (out_id, addr, val)
        let mut ms_writes:  Vec<(usize, u64)> = Vec::new();       // (out_id, val)

        for comp in &self.components {
            match comp {
                Computational::Register(reg) => {
                    let intf = reg.reflect();
                    if read_bit(&self.wire_state, &intf.inputs["write"]) {
                        let val = read_bus(&self.wire_state, &intf.inputs["data_in"]);
                        reg_updates.push((wire_id(&intf.outputs["data_out"]), val));
                    }
                }
                Computational::RAM(ram) => {
                    let intf = ram.reflect();
                    let out_id = wire_id(&intf.outputs["data_out"]);
                    // Write uses the addr computed in this cycle's initial evaluate().
                    if read_bit(&self.wire_state, &intf.inputs["write"]) {
                        let addr = read_bus(&self.wire_state, &intf.inputs["addr"]);
                        let val = read_bus(&self.wire_state, &intf.inputs["data_in"]);
                        ram_writes.push((out_id, addr, val));
                    }
                }
                Computational::MemorySystem(ms) => {
                    let intf = ms.reflect();
                    let out_id = wire_id(&intf.outputs["data_out"]);
                    if read_bit(&self.wire_state, &intf.inputs["write"]) {
                        let val = read_bus(&self.wire_state, &intf.inputs["data_in"]);
                        ms_writes.push((out_id, val));
                    }
                }
                _ => {}
            }
        }

        for (id, val) in reg_updates {
            self.reg_state.insert(id, val);
        }
        for (out_id, addr, val) in ram_writes {
            if let Some(BusResident::RAM(h)) = self.bus_residents.iter()
                .find(|res| matches!(res, BusResident::RAM(h) if h.wire_id == out_id))
            {
                h.poke(addr, val);
            }
        }
        // MS write uses device's currently-latched addr (from previous cycle).
        for (out_id, val) in ms_writes {
            if let Some(h) = self.ms_handles.iter().find(|h| h.wire_id == out_id) {
                let _ = h.device.borrow_mut().write(val);
            }
        }

        // Latch MS addr from the initial wire_state (Nand-computed from current inputs and
        // reg_state) so the re-evaluate below shows the correct memory data.
        for comp in &self.components {
            if let Computational::MemorySystem(ms) = comp {
                let intf = ms.reflect();
                let out_id = wire_id(&intf.outputs["data_out"]);
                let new_addr = read_bus(&self.wire_state, &intf.inputs["addr"]);
                if let Some(h) = self.ms_handles.iter_mut().find(|h| h.wire_id == out_id) {
                    let _ = h.device.borrow_mut().set_addr(new_addr as usize);
                    h.device.borrow_mut().ticktock();
                }
            }
        }

        // Re-evaluate with updated registers, writes, and new MS latched addr.
        self.evaluate();
        self.dirty = false;

        // Latch ROM addr after re-evaluate so the next cycle processes the *current*
        // instruction, which lets the CPU's feed-forward next_addr_mux set the right MS
        // addr latch for the cycle after.
        for comp in &self.components {
            if let Computational::ROM(rom) = comp {
                let intf = rom.reflect();
                let out_id = wire_id(&intf.outputs["out"]);
                let new_addr = read_bus(&self.wire_state, &intf.inputs["addr"]);
                if let Some(BusResident::ROM(h)) = self.bus_residents.iter()
                    .find(|res| matches!(res, BusResident::ROM(h) if h.wire_id == out_id))
                {
                    let _ = h.inner.borrow_mut().set_addr(new_addr as usize);
                }
            }
        }
    }

    fn evaluate(&mut self) {
        let mut ws: HashMap<usize, u64> = HashMap::new();

        // Seed chip inputs.
        for (name, &val) in &self.input_vals {
            if let Some(b) = self.intf.inputs.get(name) {
                write_bus(&mut ws, b, val);
            }
        }

        // Seed register outputs.
        for (&id, &val) in &self.reg_state {
            ws.insert(id, val);
        }

        // Seed RAM/ROM/MS outputs from their current addr input.
        // The addr wire is either an external chip input (seeded above) or a register output
        // (seeded from reg_state above), so it's available in ws before the Nand passes.
        for comp in &self.components {
            match comp {
                Computational::RAM(ram) => {
                    let intf = ram.reflect();
                    let out_id = wire_id(&intf.outputs["data_out"]);
                    let addr = read_bus(&ws, &intf.inputs["addr"]);
                    let val = self.bus_residents.iter()
                        .find_map(|res| match res {
                            BusResident::RAM(h) if h.wire_id == out_id => Some(h.peek(addr)),
                            _ => None,
                        })
                        .unwrap_or(0);
                    write_bus(&mut ws, &intf.outputs["data_out"], val);
                }
                Computational::ROM(rom) => {
                    let intf = rom.reflect();
                    let out_id = wire_id(&intf.outputs["out"]);
                    let val = self.bus_residents.iter()
                        .find_map(|res| match res {
                            BusResident::ROM(h) if h.wire_id == out_id =>
                                Some(h.inner.borrow().read().unwrap_or(0)),
                            _ => None,
                        })
                        .unwrap_or(0);
                    write_bus(&mut ws, &intf.outputs["out"], val);
                }
                Computational::MemorySystem(ms) => {
                    let intf = ms.reflect();
                    let out_id = wire_id(&intf.outputs["data_out"]);
                    // Read from device's currently-latched addr (device handles routing).
                    let val = self.ms_handles.iter()
                        .find_map(|h| if h.wire_id == out_id {
                            Some(h.device.borrow().read().unwrap_or(0))
                        } else {
                            None
                        })
                        .unwrap_or(0);
                    write_bus(&mut ws, &intf.outputs["data_out"], val);
                }
                _ => {}
            }
        }

        // Two Nand passes: first propagates RAM/ROM outputs through memory logic
        // (e.g. MemorySystem muxes), second lets downstream gates (ALU) use the
        // correctly computed values. Needed because component order puts CPU before
        // MemorySystem in the flattened list.
        eval_nands(&mut ws, &self.components);
        eval_nands(&mut ws, &self.components);

        self.wire_state = ws;
    }
}

fn eval_nands(ws: &mut HashMap<usize, u64>, components: &[Computational16]) {
    for comp in components {
        if let Computational::Nand(nand) = comp {
            let intf = nand.reflect();
            let a = read_bit(ws, &intf.inputs["a"]);
            let b = read_bit(ws, &intf.inputs["b"]);
            write_bit(ws, &intf.outputs["out"], !(a & b));
        }
    }
}

fn wire_id(busref: &BusRef) -> usize {
    Rc::as_ptr(&busref.id) as usize
}

fn width_mask(width: usize) -> u64 {
    if width >= 64 { u64::MAX } else { (1u64 << width) - 1 }
}

fn read_bus(ws: &HashMap<usize, u64>, b: &BusRef) -> u64 {
    let raw = ws.get(&wire_id(b)).copied().unwrap_or(0);
    (raw >> b.offset) & width_mask(b.width)
}

fn write_bus(ws: &mut HashMap<usize, u64>, b: &BusRef, value: u64) {
    let mask = width_mask(b.width);
    let entry = ws.entry(wire_id(b)).or_insert(0);
    *entry = (*entry & !(mask << b.offset)) | ((value & mask) << b.offset);
}

fn read_bit(ws: &HashMap<usize, u64>, b: &BusRef) -> bool {
    (ws.get(&wire_id(b)).copied().unwrap_or(0) >> b.offset) & 1 != 0
}

fn write_bit(ws: &mut HashMap<usize, u64>, b: &BusRef, value: bool) {
    let entry = ws.entry(wire_id(b)).or_insert(0);
    let bit = 1u64 << b.offset;
    if value { *entry |= bit; } else { *entry &= !bit; }
}

/// A view into a contiguous RAM region, using region-local addresses.
///
/// `peek(0)` returns the first word of the region regardless of its base address in the map.
#[derive(Clone)]
pub struct RegionHandle {
    pub base: usize,
    inner: RAMHandle,
}

impl RegionHandle {
    pub fn new(inner: RAMHandle) -> Self {
        let base = inner.base;
        RegionHandle { base, inner }
    }
    pub fn peek(&self, addr: u64) -> u64    { self.inner.peek(addr) }
    pub fn poke(&self, addr: u64, val: u64) { self.inner.poke(addr, val) }
    pub fn size(&self) -> usize             { self.inner.size() }
}

/// Access to auxiliary devices "on the bus" which the harness needs to inspect.
pub enum BusResident {
    RAM(RAMHandle),
    ROM(ROMHandle),
    // Future: Keyboard(KeyboardHandle),
    // Future: TTY(TTYHandle),
}

/// Private: simulation state for a MemorySystem component.
struct MSHandle {
    wire_id: usize,
    device: Rc<RefCell<MSDevice>>,
}

/// A clonable handle to a RAM instance (standalone or a region within a MemorySystem).
///
/// `wire_id` is the component's output wire id for standalone RAM, or 0 for MemorySystem regions.
/// `base` is the region's base address in the memory map (0 for standalone RAM).
#[derive(Clone)]
pub struct RAMHandle {
    pub(crate) wire_id: usize,
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
    wire_id: usize,
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
/// All actual data storage lives in [`device::RAM`] instances created by the simulator.
pub struct MemoryMap {
    pub contents: Vec<RAMMap>,
}

impl MemoryMap {
    pub fn new(contents: Vec<RAMMap>) -> Self {
        MemoryMap { contents }
    }
}
