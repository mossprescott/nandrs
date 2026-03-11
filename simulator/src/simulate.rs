use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use crate::declare::{BusRef, IC, Interface, Reflect as _};
use crate::component::{Computational, Computational16};

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
                bus_residents.push(BusResident::RAM(RAMHandle(Rc::new(RefCell::new(RAMState {
                    wire_id: wire_id(&intf.outputs["data_out"]),
                    size: ram.size,
                    data: vec![0u64; ram.size],
                })))));
            }
            Computational::ROM(rom) => {
                let intf = rom.reflect();
                bus_residents.push(BusResident::ROM(ROMHandle(Rc::new(RefCell::new(ROMState {
                    wire_id: wire_id(&intf.outputs["out"]),
                    size: rom.size,
                    data: vec![0u64; rom.size],
                    latched_addr: 0,
                })))));
            }
            Computational::MemorySystem(ms) => {
                let intf = ms.reflect();
                bus_residents.push(BusResident::MemorySystem(MemorySystemHandle::new(
                    wire_id(&intf.outputs["data_out"]),
                    memory_map.take().expect("only one MemorySystem supported"),
                )));
            }
            _ => {}
        }
    }
    let mut state = ChipState {
        intf: chip.reflect(),
        // name: chip.name().to_string(),
        components,
        input_vals: HashMap::new(),
        wire_state: HashMap::new(),
        reg_state,
        bus_residents,
        dirty: false,
    };
    state.evaluate();
    state
}

/// Runtime state of a simulated chip, and access to its inputs and outputs.
pub struct ChipState {
    intf: Interface,
    //name: String,
    components: Vec<Computational16>,
    input_vals: HashMap<String, u64>,
    wire_state: HashMap<usize, u64>,
    reg_state: HashMap<usize, u64>,
    bus_residents: Vec<BusResident>,
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
        let mut ms_writes:  Vec<(usize, u64, u64)> = Vec::new();  // (out_id, addr, val)

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
                        let addr = read_bus(&self.wire_state, &intf.inputs["addr"]);
                        let val  = read_bus(&self.wire_state, &intf.inputs["data_in"]);
                        ms_writes.push((out_id, addr, val));
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
                .find(|res| matches!(res, BusResident::RAM(h) if h.0.borrow().wire_id == out_id))
            {
                h.poke(addr, val);
            }
        }
        for (out_id, addr, val) in ms_writes {
            if let Some(BusResident::MemorySystem(h)) = self.bus_residents.iter()
                .find(|res| matches!(res, BusResident::MemorySystem(h) if h.wire_id == out_id))
            {
                h.poke(addr, val);
            }
        }

        // Latch MS addr from the initial wire_state (Nand-computed from current inputs and
        // reg_state) so the re-evaluate below shows the correct memory data.  This makes
        // same-cycle write-then-read work: the addr presented this cycle is already in
        // wire_state, so re-evaluate peeks the right location.
        for comp in &self.components {
            if let Computational::MemorySystem(ms) = comp {
                let intf = ms.reflect();
                let out_id = wire_id(&intf.outputs["data_out"]);
                let new_addr = read_bus(&self.wire_state, &intf.inputs["addr"]);
                if let Some(BusResident::MemorySystem(h)) = self.bus_residents.iter_mut()
                    .find(|res| matches!(res, BusResident::MemorySystem(h) if h.wire_id == out_id))
                {
                    h.latched_addr = new_addr;
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
                    .find(|res| matches!(res, BusResident::ROM(h) if h.0.borrow().wire_id == out_id))
                {
                    h.0.borrow_mut().latched_addr = new_addr;
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
                            BusResident::RAM(h) if h.0.borrow().wire_id == out_id => {
                                h.0.borrow().data.get(addr as usize).copied()
                            }
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
                            BusResident::ROM(h) if h.0.borrow().wire_id == out_id => {
                                let s = h.0.borrow();
                                s.data.get(s.latched_addr as usize).copied()
                            }
                            _ => None,
                        })
                        .unwrap_or(0);
                    write_bus(&mut ws, &intf.outputs["out"], val);
                }
                Computational::MemorySystem(ms) => {
                    let intf = ms.reflect();
                    let out_id = wire_id(&intf.outputs["data_out"]);
                    let val = self.bus_residents.iter()
                        .find_map(|res| match res {
                            BusResident::MemorySystem(h) if h.wire_id == out_id => {
                                Some(h.peek(h.latched_addr))
                            }
                            _ => None,
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

/// A view into a contiguous region of a MemorySystem, using region-local addresses.
///
/// `peek(0)` returns the first word of the region regardless of its base address.
#[derive(Clone)]
pub struct RegionHandle {
    inner: MemorySystemHandle,
    pub base: u64,
}

impl RegionHandle {
    pub fn new(inner: MemorySystemHandle, base: u64) -> Self { RegionHandle { inner, base } }
    pub fn peek(&self, addr: u64) -> u64       { self.inner.peek(self.base + addr) }
    pub fn poke(&self, addr: u64, val: u64)    { self.inner.poke(self.base + addr, val) }
    pub fn size(&self) -> usize                { self.inner.size() }
}

/// Access to auxiliary chips "on the bus", which the harness needs to access.
/// Which components – and how many of each – are present depends on the chip design.
///
/// Realistically, there will be two RAMs present if the conventional HACK MemorySystem is used.
/// In that case, the RAM sizes will differ.
pub enum BusResident {
    RAM(RAMHandle),
    ROM(ROMHandle),
    MemorySystem(MemorySystemHandle),
    // Keyboard(KeyboardHandle),
    // TTY(TTYHandle),
}

/// Track the contents and simulation state of a RAM.
///
/// Every word is initialized to 0.
///
/// Future: for versimillitude, each RAM should have a defined read latency. The address input
/// will have to be presented one or more cycles *before* the output value is actually used. For
/// example, the HACK CPU always latches the address into register A (and presents it to the RAM)
/// at least one cycle before any read/write. In "realistic" designs, one or two cycles or latency
/// was typical, even decades ago.
pub struct RAMState {
    wire_id: usize,
    pub size: usize,
    data: Vec<u64>,
}

impl RAMState {
    /// Read a word. If the address is out-of-range, returns 0.
    pub fn peek(&self, addr: u64) -> u64 {
        self.data.get(addr as usize).copied().unwrap_or_else(|| {
            println!("Out of range read: RAM[{}]", addr);
            0
        })
    }

    /// Write a word. If the address is out-of-range, ignore.
    pub fn poke(&mut self, addr: u64, val: u64) {
        if let Some(cell) = self.data.get_mut(addr as usize) {
            *cell = val;
        } else {
            println!("Out of range write: {} -> RAM[{}]", val, addr);
        }
    }
}

/// Hold pre-initialized ROM contents.
pub struct ROMState {
    wire_id: usize,
    pub size: usize,
    data: Vec<u64>,
    latched_addr: u64,
}

impl ROMState {
    /// Read a word. If the address is out-of-range, returns 0.
    pub fn read(&self, addr: u64) -> u64 {
        self.data.get(addr as usize).copied().unwrap_or_else(|| {
            println!("Out of range read: ROM[{}]", addr);
            0
        })
    }

    /// Replace the entire contents.
    pub fn flash(&mut self, data: Vec<u64>) {
        self.data = data;
    }
}

/// Trait for custom memory system implementations supplied at simulation time.
pub trait MemorySystemState {
    fn peek(&self, addr: u64) -> u64;
    fn poke(&mut self, addr: u64, val: u64);
    fn size(&self) -> usize;
}

/// Descriptor for one contiguous RAM region in a memory map.
pub struct RAMMap {
    pub size: usize,
    pub base: usize,
}

/// A multi-region memory system that implements [`MemorySystemState`].
///
/// Constructed from a list of [`RAMMap`] region descriptors; all regions are zero-initialized.
/// Addresses outside every declared region read as 0 and writes are silently dropped.
///
/// When regions' address spaces overlap, they're applied top-to-bottom; the first region in
/// `contents` that can handle an address gets used.
///
/// FIXME: this struct should just define the mapping. Some internal type (MemorySystemState?)
/// will hold onto the values and handle access.
pub struct MemoryMap {
    pub contents: Vec<RAMMap>,
    data: Vec<Vec<u64>>,
}

impl MemoryMap {
    pub fn new(contents: Vec<RAMMap>) -> Self {
        let data = contents.iter().map(|r| vec![0u64; r.size]).collect();
        MemoryMap { contents, data }
    }

    /// Peek directly into region `i` (bypasses address translation).
    pub fn peek_region(&self, region: usize, addr: u64) -> u64 {
        self.data[region].get(addr as usize).copied().unwrap_or(0)
    }

    /// Poke directly into region `i` (bypasses address translation).
    pub fn poke_region(&mut self, region: usize, addr: u64, val: u64) {
        if let Some(cell) = self.data[region].get_mut(addr as usize) {
            *cell = val;
        }
    }
}

impl MemorySystemState for MemoryMap {
    fn peek(&self, addr: u64) -> u64 {
        let a = addr as usize;
        for (i, r) in self.contents.iter().enumerate() {
            if a >= r.base && a < r.base + r.size {
                return self.data[i][a - r.base];
            }
        }
        // TODO: record/report these in some legit way. Tests might want to fail, for example.
        eprintln!("Bus error: no region when reading from location {} (0x{:04X})", addr, addr);
        0
    }

    fn poke(&mut self, addr: u64, val: u64) {
        let a = addr as usize;
        for (i, r) in self.contents.iter().enumerate() {
            if a >= r.base && a < r.base + r.size {
                self.data[i][a - r.base] = val;
                return;
            }
        }
        // TODO: record/report these in some legit way. Tests might want to fail, for example.
        eprintln!("Bus error: no region when writing to location {} (0x{:04X}) <- {} (0x{:04X})", addr, addr, val, val);
    }

    /// TODO: this doesn't seem useful. Maybe should report the range of valid addresses? Seems
    /// like it's always contiguous, at least for Hack and pynand's BigComputer.
    fn size(&self) -> usize {
        self.contents.iter().map(|r| r.base + r.size).max().unwrap_or(0)
    }
}

/// A clonable handle to a MemorySystem implementation in the simulated circuit.
#[derive(Clone)]
pub struct MemorySystemHandle {
    pub(crate) wire_id: usize,
    pub(crate) latched_addr: u64,
    inner: Rc<RefCell<dyn MemorySystemState>>,
}

impl MemorySystemHandle {
    pub fn new(wire_id: usize, state: impl MemorySystemState + 'static) -> Self {
        MemorySystemHandle { wire_id, latched_addr: 0, inner: Rc::new(RefCell::new(state)) }
    }
    pub fn peek(&self, addr: u64) -> u64       { self.inner.borrow().peek(addr) }
    pub fn poke(&self, addr: u64, val: u64)    { self.inner.borrow_mut().poke(addr, val) }
    pub fn size(&self) -> usize                { self.inner.borrow().size() }
}

/// A clonable handle to a RAM instance in the simulated circuit.
/// Cloning the handle gives another reference to the same underlying RAM.
#[derive(Clone)]
pub struct RAMHandle(Rc<RefCell<RAMState>>);

impl RAMHandle {
    pub fn peek(&self, addr: u64) -> u64       { self.0.borrow().peek(addr) }
    pub fn poke(&self, addr: u64, val: u64)    { self.0.borrow_mut().poke(addr, val) }
    pub fn size(&self) -> usize                { self.0.borrow().size }
}

/// A clonable handle to a ROM instance in the simulated circuit.
#[derive(Clone)]
pub struct ROMHandle(Rc<RefCell<ROMState>>);

impl ROMHandle {
    pub fn read(&self, addr: u64) -> u64       { self.0.borrow().read(addr) }
    pub fn flash(&self, data: Vec<u64>)        { self.0.borrow_mut().flash(data) }
    pub fn size(&self) -> usize                { self.0.borrow().size }
}