use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use crate::device::MemoryDevice as _;

use super::{ChipWiring, wiring};

type DeviceRAM = Rc<RefCell<crate::device::RAM>>;
type MSDevice   = crate::device::MemorySystem<DeviceRAM>;

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
                        self.reg_state[reg.data_out.0 as usize] = self.wire_state[reg.data_in.0 as usize];
                    }
                }
                wiring::ComponentWiring::RAM(ram) => {
                    if read_bit(&self.wire_state, ram.write) {
                        let _ = self.ram_devices[ram.device_slot].borrow_mut().write(self.wire_state[ram.data_in.0 as usize]);
                    }
                }
                wiring::ComponentWiring::MemorySystem(ms) => {
                    if read_bit(&self.wire_state, ms.write) {
                        let _ = self.ms_devices[ms.device_slot].borrow_mut().write(self.wire_state[ms.data_in.0 as usize]);
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
                    let _ = self.ram_devices[ram.device_slot].borrow_mut().set_addr(self.wire_state[ram.addr.0 as usize] as usize);
                    self.ram_devices[ram.device_slot].borrow_mut().ticktock();
                }
                wiring::ComponentWiring::MemorySystem(ms) => {
                    let _ = self.ms_devices[ms.device_slot].borrow_mut().set_addr(self.wire_state[ms.addr.0 as usize] as usize);
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
                let _ = self.rom_devices[rom.device_slot].borrow_mut().set_addr(self.wire_state[rom.addr.0 as usize] as usize);
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
                    self.wire_state[ram.out.0 as usize] = self.ram_devices[ram.device_slot].borrow().read().unwrap_or(0);
                }
                wiring::ComponentWiring::ROM(rom) => {
                    self.wire_state[rom.out.0 as usize] = self.rom_devices[rom.device_slot].borrow().read().unwrap_or(0);
                }
                wiring::ComponentWiring::MemorySystem(ms) => {
                    self.wire_state[ms.out.0 as usize] = self.ms_devices[ms.device_slot].borrow().read().unwrap_or(0);
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
        match comp {
            // This variant is cheaper to evaluate; just one bit without masking:
            wiring::ComponentWiring::Nand(nand) => {
                let a = read_bit(ws, nand.a);
                let b = read_bit(ws, nand.b);
                write_bit(ws, nand.out, !(a & b));
            }
            // This variant gets more done in a single iteration:
            wiring::ComponentWiring::ParallelNand(nand) => {
                ws[nand.out.0 as usize] = !(ws[nand.a.0 as usize] & ws[nand.b.0 as usize]);
            }
            _ => {}
        }
    }
}

fn width_mask(width: usize) -> u64 {
    if width >= 64 { u64::MAX } else { (1u64 << width) - 1 }
}

/// Read a range of bits from a certain location. Now used only for extracting chip outputs from the
/// wire state.
fn read_bus(ws: &[u64], b: wiring::WireRef) -> u64 {
    (ws[b.id.0 as usize] >> b.offset) & width_mask(b.width as usize)
}

/// Write a range of bits into a certain location. Now used only for injecting chip inputs into the
/// initial wire state.
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
