use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use crate::declare::{BusRef, IC, Interface, Reflect as _};
use crate::component::{Computational, Computational16};

/// Transform circuit description for simulation.
///
/// Note: currently 16-bit words are assumed, but up to 64-bits wouldn't be a problem if the type
/// was generalized.
pub fn synthesize<C>(chip: &IC<C>) -> ChipState
where
    C: Clone + crate::Reflect + Into<Computational16>,
{
    let components: Vec<Computational16> = chip.components.iter().cloned().map(Into::into).collect();
    let mut reg_state: HashMap<usize, u64> = HashMap::new();
    let mut bus_residents: Vec<BusResident> = Vec::new();
    for comp in &components {
        match comp {
            Computational::Register(reg) => {
                let intf = reg.reflect();
                assert_eq!(intf.outputs["out"].width, 16);
                reg_state.insert(wire_id(&intf.outputs["out"]), 0);
            }
            Computational::RAM(ram) => {
                let intf = ram.reflect();
                assert_eq!(intf.outputs["out"].width, 16);
                bus_residents.push(BusResident::RAM(RAMHandle(Rc::new(RefCell::new(RAMState {
                    wire_id: wire_id(&intf.outputs["out"]),
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
                })))));
            }
            _ => {}
        }
    }
    let mut state = ChipState {
        intf: chip.reflect(),
        name: chip.name().to_string(),
        components,
        input_vals: HashMap::new(),
        wire_state: HashMap::new(),
        reg_state,
        bus_residents,
    };
    state.evaluate();
    state
}

/// Runtime state of a simulated chip, and access to its inputs and outputs.
pub struct ChipState {
    intf: Interface,
    name: String,
    components: Vec<Computational16>,
    input_vals: HashMap<String, u64>,
    wire_state: HashMap<usize, u64>,
    reg_state: HashMap<usize, u64>,
    bus_residents: Vec<BusResident>,
}

impl ChipState {
    /// Set the value of an input for the next cycle.
    pub fn set(&mut self, name: &str, value: u64) {
        self.input_vals.insert(name.to_string(), value);
    }

    /// Get the value of an output as of the last cycle.
    pub fn get(&self, name: &str) -> u64 {
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
        self.evaluate();

        // Collect updates (avoids borrow conflict between components and state maps).
        let mut reg_updates: Vec<(usize, u64)> = Vec::new();
        let mut ram_updates: Vec<(usize, u64, u64)> = Vec::new();

        for comp in &self.components {
            match comp {
                Computational::Register(reg) => {
                    let intf = reg.reflect();
                    if read_bit(&self.wire_state, &intf.inputs["load"]) {
                        let val = read_bus(&self.wire_state, &intf.inputs["data"]);
                        reg_updates.push((wire_id(&intf.outputs["out"]), val));
                    }
                }
                Computational::RAM(ram) => {
                    let intf = ram.reflect();
                    if read_bit(&self.wire_state, &intf.inputs["load"]) {
                        let addr = read_bus(&self.wire_state, &intf.inputs["addr"]);
                        let val  = read_bus(&self.wire_state, &intf.inputs["data"]);
                        ram_updates.push((wire_id(&intf.outputs["out"]), addr, val));
                    }
                }
                _ => {}
            }
        }

        for (id, val) in reg_updates {
            self.reg_state.insert(id, val);
        }
        for (out_id, addr, val) in ram_updates {
            if let Some(BusResident::RAM(h)) = self.bus_residents.iter()
                .find(|res| matches!(res, BusResident::RAM(h) if h.0.borrow().wire_id == out_id))
            {
                h.poke(addr, val);
            }
        }

        // Re-evaluate so outputs reflect the new state.
        self.evaluate();
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

        // First Nand pass — computes addresses needed for RAM/ROM lookup.
        eval_nands(&mut ws, &self.components);

        // Seed RAM/ROM outputs based on computed addresses.
        for comp in &self.components {
            match comp {
                Computational::RAM(ram) => {
                    let intf = ram.reflect();
                    let addr   = read_bus(&ws, &intf.inputs["addr"]);
                    let out_id = wire_id(&intf.outputs["out"]);
                    let val = self.bus_residents.iter()
                        .find_map(|res| match res {
                            BusResident::RAM(h) if h.0.borrow().wire_id == out_id => h.0.borrow().data.get(addr as usize).copied(),
                            _ => None,
                        })
                        .unwrap_or(0);
                    write_bus(&mut ws, &intf.outputs["out"], val);
                }
                Computational::ROM(rom) => {
                    let intf = rom.reflect();
                    let addr   = read_bus(&ws, &intf.inputs["addr"]);
                    let out_id = wire_id(&intf.outputs["out"]);
                    let val = self.bus_residents.iter()
                        .find_map(|res| match res {
                            BusResident::ROM(h) if h.0.borrow().wire_id == out_id => h.0.borrow().data.get(addr as usize).copied(),
                            _ => None,
                        })
                        .unwrap_or(0);
                    write_bus(&mut ws, &intf.outputs["out"], val);
                }
                _ => {}
            }
        }

        // Second Nand pass — uses RAM/ROM outputs.
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

/// Access to auxiliary chips "on the bus", which the harness needs to access.
/// Which components – and how many of each – are present depends on the chip design.
///
/// Realistically, there will be two RAMs present if the conventional HACK MemorySystem is used.
/// In that case, the RAM sizes will differ.
pub enum BusResident {
    RAM(RAMHandle),
    ROM(ROMHandle),
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