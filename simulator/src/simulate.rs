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
    let mut reg_state: HashMap<WireID, u64> = HashMap::new();
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
                    bus_residents.push(BusResident::RAM(RAMHandle { wire_id: WireID(0), base: r.base, inner: Rc::clone(&ram) }));
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
    let component_wiring: Vec<wiring::ComponentWiring> = components.iter().map(|comp| {
        use wiring::ComponentWiring as CW;
        match comp {
            Computational::Nand(c)         => CW::Nand(wiring::NandWiring::new(c)),
            Computational::Register(c)     => CW::Register(wiring::RegisterWiring::new(c)),
            Computational::RAM(c)          => CW::RAM(wiring::RAMWiring::new(c)),
            Computational::ROM(c)          => CW::ROM(wiring::ROMWiring::new(c)),
            Computational::MemorySystem(c) => CW::MemorySystem(wiring::MemorySystemWiring::new(c)),
            Computational::Const(_)        => CW::Const,
        }
    }).collect();

    let mut state = ChipState {
        intf: chip.reflect(),
        component_wiring,
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
    component_wiring: Vec<wiring::ComponentWiring>,

    input_vals: HashMap<String, u64>,
    dirty: bool,

    wire_state: HashMap<WireID, u64>,
    reg_state: HashMap<WireID, u64>,

    bus_residents: Vec<BusResident>,
    ms_handles: Vec<MSHandle>,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct WireID(usize);

/// Pre-computed wiring info about components, used during evaluation.
mod wiring {
    use crate::component::{Nand, Register16, RAM16, ROM16, MemorySystem16};
    use crate::declare::{BusRef, Reflect};
    use super::{WireID, wire_id};

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

    pub(super) struct BitRef { pub(super) id: WireID, pub(super) offset: usize }
    impl BitRef {
        pub(super) fn from(b: &BusRef) -> Self { BitRef { id: wire_id(b), offset: b.offset } }
    }

    pub(super) struct WireRef { pub(super) id: WireID, pub(super) offset: usize, pub(super) width: usize }
    impl WireRef {
        pub(super) fn from(b: &BusRef) -> Self { WireRef { id: wire_id(b), offset: b.offset, width: b.width } }
    }

    pub(super) struct NandWiring { pub(super) a: BitRef, pub(super) b: BitRef, pub(super) out: BitRef }
    impl NandWiring {
        pub(super) fn new(nand: &Nand) -> Self {
            let intf = nand.reflect();
            Self {
                a:   BitRef::from(&intf.inputs["a"]),
                b:   BitRef::from(&intf.inputs["b"]),
                out: BitRef::from(&intf.outputs["out"]),
            }
        }
    }

    pub(super) struct RegisterWiring { pub(super) write: BitRef, pub(super) data_in: WireRef, pub(super) data_out: WireID }
    impl RegisterWiring {
        pub(super) fn new(reg: &Register16) -> Self {
            let intf = reg.reflect();
            Self {
                write:    BitRef::from(&intf.inputs["write"]),
                data_in:  WireRef::from(&intf.inputs["data_in"]),
                data_out: wire_id(&intf.outputs["data_out"]),
            }
        }
    }

    pub(super) struct ROMWiring { pub(super) out: WireRef, pub(super) addr: WireRef }
    impl ROMWiring {
        pub(super) fn new(rom: &ROM16) -> Self {
            let intf = rom.reflect();
            Self {
                out:  WireRef::from(&intf.outputs["out"]),
                addr: WireRef::from(&intf.inputs["addr"]),
            }
        }
    }

    pub(super) struct RAMWiring { pub(super) out: WireRef, pub(super) addr: WireRef, pub(super) write: BitRef, pub(super) data_in: WireRef }
    impl RAMWiring {
        pub(super) fn new(ram: &RAM16) -> Self {
            let intf = ram.reflect();
            Self {
                out:     WireRef::from(&intf.outputs["data_out"]),
                addr:    WireRef::from(&intf.inputs["addr"]),
                write:   BitRef::from(&intf.inputs["write"]),
                data_in: WireRef::from(&intf.inputs["data_in"]),
            }
        }
    }

    pub(super) struct MemorySystemWiring { pub(super) out: WireRef, pub(super) addr: WireRef, pub(super) write: BitRef, pub(super) data_in: WireRef }
    impl MemorySystemWiring {
        pub(super) fn new(ms: &MemorySystem16) -> Self {
            let intf = ms.reflect();
            Self {
                out:     WireRef::from(&intf.outputs["data_out"]),
                addr:    WireRef::from(&intf.inputs["addr"]),
                write:   BitRef::from(&intf.inputs["write"]),
                data_in: WireRef::from(&intf.inputs["data_in"]),
            }
        }
    }
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
            .map(|b| read_bus(&self.wire_state, &wiring::WireRef::from(b)))
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
        let mut reg_updates: Vec<(WireID, u64)> = Vec::new();
        let mut ram_writes: Vec<(WireID, u64, u64)> = Vec::new();  // (out_id, addr, val)
        let mut ms_writes:  Vec<(WireID, u64)> = Vec::new();       // (out_id, val)

        for comp in &self.component_wiring {
            match comp {
                wiring::ComponentWiring::Register(reg) => {
                    if read_bit(&self.wire_state, &reg.write) {
                        let val = read_bus(&self.wire_state, &reg.data_in);
                        reg_updates.push((reg.data_out, val));
                    }
                }
                wiring::ComponentWiring::RAM(ram) => {
                    // Write uses the addr computed in this cycle's initial evaluate().
                    if read_bit(&self.wire_state, &ram.write) {
                        let addr = read_bus(&self.wire_state, &ram.addr);
                        let val  = read_bus(&self.wire_state, &ram.data_in);
                        ram_writes.push((ram.out.id, addr, val));
                    }
                }
                wiring::ComponentWiring::MemorySystem(ms) => {
                    if read_bit(&self.wire_state, &ms.write) {
                        let val = read_bus(&self.wire_state, &ms.data_in);
                        ms_writes.push((ms.out.id, val));
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
        for comp in &self.component_wiring {
            if let wiring::ComponentWiring::MemorySystem(ms) = comp {
                let new_addr = read_bus(&self.wire_state, &ms.addr);
                if let Some(h) = self.ms_handles.iter_mut().find(|h| h.wire_id == ms.out.id) {
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
        for comp in &self.component_wiring {
            if let wiring::ComponentWiring::ROM(rom) = comp {
                let new_addr = read_bus(&self.wire_state, &rom.addr);
                if let Some(BusResident::ROM(h)) = self.bus_residents.iter()
                    .find(|res| matches!(res, BusResident::ROM(h) if h.wire_id == rom.out.id))
                {
                    let _ = h.inner.borrow_mut().set_addr(new_addr as usize);
                }
            }
        }
    }

    fn evaluate(&mut self) {
        let mut ws: HashMap<WireID, u64> = HashMap::new();

        // Seed chip inputs.
        for (name, &val) in &self.input_vals {
            if let Some(b) = self.intf.inputs.get(name) {
                write_bus(&mut ws, &wiring::WireRef::from(b), val);
            }
        }

        // Seed register outputs.
        for (&id, &val) in &self.reg_state {
            ws.insert(id, val);
        }

        // Seed RAM/ROM/MS outputs from their current addr input.
        // The addr wire is either an external chip input (seeded above) or a register output
        // (seeded from reg_state above), so it's available in ws before the Nand passes.
        for comp in &self.component_wiring {
            match comp {
                wiring::ComponentWiring::RAM(ram) => {
                    let addr = read_bus(&ws, &ram.addr);
                    let val = self.bus_residents.iter()
                        .find_map(|res| match res {
                            BusResident::RAM(h) if h.wire_id == ram.out.id => Some(h.peek(addr)),
                            _ => None,
                        })
                        .unwrap_or(0);
                    write_bus(&mut ws, &ram.out, val);
                }
                wiring::ComponentWiring::ROM(rom) => {
                    let val = self.bus_residents.iter()
                        .find_map(|res| match res {
                            BusResident::ROM(h) if h.wire_id == rom.out.id =>
                                Some(h.inner.borrow().read().unwrap_or(0)),
                            _ => None,
                        })
                        .unwrap_or(0);
                    write_bus(&mut ws, &rom.out, val);
                }
                wiring::ComponentWiring::MemorySystem(ms) => {
                    // Read from device's currently-latched addr (device handles routing).
                    let val = self.ms_handles.iter()
                        .find_map(|h| if h.wire_id == ms.out.id {
                            Some(h.device.borrow().read().unwrap_or(0))
                        } else {
                            None
                        })
                        .unwrap_or(0);
                    write_bus(&mut ws, &ms.out, val);
                }
                _ => {}
            }
        }

        // Two Nand passes: first propagates RAM/ROM outputs through memory logic
        // (e.g. MemorySystem muxes), second lets downstream gates (ALU) use the
        // correctly computed values. Needed because component order puts CPU before
        // MemorySystem in the flattened list.
        eval_nands(&mut ws, &self.component_wiring);
        eval_nands(&mut ws, &self.component_wiring);

        self.wire_state = ws;
    }
}

fn eval_nands(ws: &mut HashMap<WireID, u64>, component_wiring: &[wiring::ComponentWiring]) {
    for comp in component_wiring {
        if let wiring::ComponentWiring::Nand(nand) = comp {
            let a = read_bit(ws, &nand.a);
            let b = read_bit(ws, &nand.b);
            write_bit(ws, &nand.out, !(a & b));
        }
    }
}

fn wire_id(busref: &BusRef) -> WireID {
    WireID(Rc::as_ptr(&busref.id) as usize)
}

fn width_mask(width: usize) -> u64 {
    if width >= 64 { u64::MAX } else { (1u64 << width) - 1 }
}

fn read_bus(ws: &HashMap<WireID, u64>, b: &wiring::WireRef) -> u64 {
    let raw = ws.get(&b.id).copied().unwrap_or(0);
    (raw >> b.offset) & width_mask(b.width)
}

fn write_bus(ws: &mut HashMap<WireID, u64>, b: &wiring::WireRef, value: u64) {
    let mask = width_mask(b.width);
    let entry = ws.entry(b.id).or_insert(0);
    *entry = (*entry & !(mask << b.offset)) | ((value & mask) << b.offset);
}

fn read_bit(ws: &HashMap<WireID, u64>, b: &wiring::BitRef) -> bool {
    (ws.get(&b.id).copied().unwrap_or(0) >> b.offset) & 1 != 0
}

fn write_bit(ws: &mut HashMap<WireID, u64>, b: &wiring::BitRef, value: bool) {
    let entry = ws.entry(b.id).or_insert(0);
    let bit = 1u64 << b.offset;
    if value { *entry |= bit; } else { *entry &= !bit; }
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
    wire_id: WireID,
    device: Rc<RefCell<MSDevice>>,
}

/// A clonable handle to a RAM instance (standalone or a region within a MemorySystem).
///
/// `wire_id` is the component's output wire id for standalone RAM, or 0 for MemorySystem regions.
/// `base` is the region's base address in the memory map (0 for standalone RAM).
#[derive(Clone)]
pub struct RAMHandle {
    wire_id: WireID,
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
    wire_id: WireID,
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
