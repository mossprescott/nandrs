use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use crate::device::MemoryDevice as _;
use crate::nat::Nat;
use crate::word::{Word, Storable};

use super::{ChipWiring, wiring};
use super::memory::RegionMap;

type DeviceRAM<A, D>    = Rc<RefCell<crate::device::RAM<A, D>>>;
type DeviceROM<A, D>    = Rc<RefCell<crate::device::ROM<A, D>>>;
type DeviceSerial<D>    = Rc<RefCell<crate::device::Serial<D>>>;

/// A device that can appear as a region within a MemorySystem's address space.
enum MSRegion<A: Nat + Storable, D: Nat + Storable> {
    RAM(DeviceRAM<A, D>),
    ROM(DeviceROM<A, D>),
    Serial(DeviceSerial<D>),
}

impl<A: Nat + Storable, D: Nat + Storable> crate::device::MemoryDevice<A, D> for MSRegion<A, D> {
    fn set_addr(&mut self, addr: Word<A>) -> Result<(), crate::device::Error> {
        match self {
            MSRegion::RAM(d)    => d.borrow_mut().set_addr(addr),
            MSRegion::ROM(d)    => d.borrow_mut().set_addr(addr),
            MSRegion::Serial(d) => d.borrow_mut().set_addr(addr),
        }
    }
    fn ticktock(&mut self) {
        match self {
            MSRegion::RAM(d)    => d.borrow_mut().ticktock(),
            MSRegion::ROM(d)    => d.borrow_mut().ticktock(),
            MSRegion::Serial(d) => <crate::device::Serial<D> as crate::device::MemoryDevice<A, D>>::ticktock(&mut *d.borrow_mut()),
        }
    }
    fn read(&self) -> Result<Word<D>, crate::device::Error> {
        match self {
            MSRegion::RAM(d)    => d.borrow().read(),
            MSRegion::ROM(d)    => d.borrow().read(),
            MSRegion::Serial(d) => <crate::device::Serial<D> as crate::device::MemoryDevice<A, D>>::read(&*d.borrow()),
        }
    }
    fn write(&mut self, word: Word<D>) -> Result<(), crate::device::Error> {
        match self {
            MSRegion::RAM(d)    => d.borrow_mut().write(word),
            MSRegion::ROM(d)    => d.borrow_mut().write(word),
            MSRegion::Serial(d) => <crate::device::Serial<D> as crate::device::MemoryDevice<A, D>>::write(&mut *d.borrow_mut(), word),
        }
    }
}

type MSDevice<A, D> = crate::device::MemorySystem<A, MSRegion<A, D>>;

/// Runtime state of a simulated chip, and access to its inputs and outputs.
///
/// Internal state is stored in u64 for efficiency; the raw bits are wrapped in the appropriate
/// `Word<A>` (addresses) and `Word<D>` (data) when they are exchanged with the outside world.
pub struct ChipState<A: Nat + Storable, D: Nat + Storable> {
    /// The graph of components, input, and outputs, and where state is to be stored.
    wiring:       ChipWiring<D>,

    ram_devices:    Vec<DeviceRAM<A, D>>,
    rom_devices:    Vec<DeviceROM<A, D>>,
    ms_devices:     Vec<Rc<RefCell<MSDevice<A, D>>>>,
    serial_devices: Vec<DeviceSerial<D>>,
    bus_residents:   Vec<BusResident<A, D>>,

    /// State of register contents as of the last clock cycle, as well as any wires holding constant
    /// values.
    reg_state:    Vec<u64>,

    /// Input value supplied from outside, for initializing the state of the wires. Note: typically
    /// the full computer has few, if any, of these inputs, so not really a factor in performance.
    input_vals:   HashMap<wiring::WireRef, u64>,

    /// When true, inputs have changed since the last time we progagated the effects to wire_state.
    dirty:        bool,

    /// State of all wires, including the outputs, as of the last cycle, so they can be inspected
    /// from outside.
    wire_state:   Vec<u64>,
}

/// Allocate simulation state (RAM/ROM buffers, registers) and run an initial evaluation.
pub fn initialize<A: Nat + Storable, D: Nat + Storable>(wiring: ChipWiring<D>) -> ChipState<A, D> {
    let n_wires = wiring.n_wires;

    let ram_devices: Vec<DeviceRAM<A, D>> = wiring.ram_specs.iter()
        .map(|s| Rc::new(RefCell::new(crate::device::RAM::new(s.size))))
        .collect();

    let rom_devices: Vec<DeviceROM<A, D>> = wiring.rom_specs.iter()
        .map(|s| Rc::new(RefCell::new(crate::device::ROM::new(s.size))))
        .collect();

    let mut ms_bus_residents: Vec<BusResident<A, D>> = Vec::new();
    let ms_devices: Vec<Rc<RefCell<MSDevice<A, D>>>> = wiring.ms_specs.iter().map(|spec| {
        let mut overlays: Vec<crate::device::Overlay<A, MSRegion<A, D>>> = Vec::new();
        for region in &spec.regions {
            match region {
                RegionMap::RAM(r) => {
                    let ram: DeviceRAM<A, D> = Rc::new(RefCell::new(crate::device::RAM::new(r.size)));
                    ms_bus_residents.push(BusResident::RAM(RAMHandle { base: r.base, inner: Rc::clone(&ram) }));
                    overlays.push(crate::device::Overlay { base: Word::new(r.base as u64), device: MSRegion::RAM(ram) });
                }
                RegionMap::ROM(r) => {
                    let rom: DeviceROM<A, D> = Rc::new(RefCell::new(crate::device::ROM::new(r.size)));
                    ms_bus_residents.push(BusResident::ROM(ROMHandle { inner: Rc::clone(&rom) }));
                    overlays.push(crate::device::Overlay { base: Word::new(r.base as u64), device: MSRegion::ROM(rom) });
                }
                RegionMap::Serial(s) => {
                    let serial: DeviceSerial<D> = Rc::new(RefCell::new(crate::device::Serial::new()));
                    ms_bus_residents.push(BusResident::Serial(SerialHandle { inner: Rc::clone(&serial) }));
                    overlays.push(crate::device::Overlay { base: Word::new(s.base as u64), device: MSRegion::Serial(serial) });
                }
            }
        }
        Rc::new(RefCell::new(MSDevice { devices: overlays }))
    }).collect();

    let serial_devices: Vec<DeviceSerial<D>> = wiring.serial_specs.iter()
        .map(|_| Rc::new(RefCell::new(crate::device::Serial::new())))
        .collect();

    let mut bus_residents: Vec<BusResident<A, D>> = Vec::new();
    for ram in &ram_devices {
        bus_residents.push(BusResident::RAM(RAMHandle { base: 0, inner: Rc::clone(ram) }));
    }
    for rom in &rom_devices {
        bus_residents.push(BusResident::ROM(ROMHandle { inner: Rc::clone(rom) }));
    }
    bus_residents.extend(ms_bus_residents);
    for serial in &serial_devices {
        bus_residents.push(BusResident::Serial(SerialHandle { inner: Rc::clone(serial) }));
    }

    let mut reg_state = vec![0u64; n_wires];
    for cw in &wiring.const_wiring {
        reg_state[cw.out.0 as usize] = cw.value;
    }

    let mut state = ChipState {
        wiring,
        ram_devices,
        rom_devices,
        ms_devices,
        serial_devices,
        bus_residents,
        reg_state,
        input_vals: HashMap::new(),
        dirty: false,
        wire_state: vec![0u64; n_wires],
    };
    state.evaluate();
    state
}

impl<A: Nat + Storable, D: Nat + Storable> ChipState<A, D> {

    /// Set the value of an input. Combinational outputs will reflect this on the next `get()`.
    pub fn set(&mut self, name: &str, value: Word<D>) {
        if let Some(&wr) = self.wiring.input_wiring.get(name) {
            self.input_vals.insert(wr, value.unsigned());
        }
        self.dirty = true;
    }

    /// Get the value of an output, re-evaluating combinational logic if any inputs changed.
    pub fn get(&mut self, name: &str) -> Word<D> {
        if self.dirty {
            self.evaluate();
            self.dirty = false;
        }
        Word::new(self.wiring.output_wiring.get(name)
            .map(|&wr| read_bus(&self.wire_state, wr))
            .unwrap_or(0))
    }

    /// RAM and ROM instances present in the simulated circuit.
    pub fn bus_residents(&self) -> &[BusResident<A, D>] {
        &self.bus_residents
    }

    /// RAM and ROM instances present in the simulated circuit, mutably (e.g. to load a ROM).
    pub fn bus_residents_mut(&mut self) -> &mut [BusResident<A, D>] {
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
                        let _ = self.ram_devices[ram.device_slot].borrow_mut().write(Word::new(self.wire_state[ram.data_in.0 as usize]));
                    }
                }
                wiring::ComponentWiring::MemorySystem(ms) => {
                    if read_bit(&self.wire_state, ms.write) {
                        let _ = self.ms_devices[ms.device_slot].borrow_mut().write(Word::new(self.wire_state[ms.data_in.0 as usize]));
                    }
                }
                wiring::ComponentWiring::Serial(s) => {
                    if read_bit(&self.wire_state, s.write) {
                        let word: Word<D> = Word::new(self.wire_state[s.data_in.0 as usize]);
                        let _ = <crate::device::Serial<D> as crate::device::MemoryDevice<A, D>>::write(&mut *self.serial_devices[s.device_slot].borrow_mut(), word);
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
                    let _ = self.ram_devices[ram.device_slot].borrow_mut().set_addr(Word::new(self.wire_state[ram.addr.0 as usize]));
                    self.ram_devices[ram.device_slot].borrow_mut().ticktock();
                }
                wiring::ComponentWiring::MemorySystem(ms) => {
                    let _ = self.ms_devices[ms.device_slot].borrow_mut().set_addr(Word::new(self.wire_state[ms.addr.0 as usize]));
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
                let _ = self.rom_devices[rom.device_slot].borrow_mut().set_addr(Word::new(self.wire_state[rom.addr.0 as usize]));
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
                    self.wire_state[ram.out.0 as usize] = self.ram_devices[ram.device_slot].borrow().read().map(|w| w.unsigned()).unwrap_or(0);
                }
                wiring::ComponentWiring::ROM(rom) => {
                    self.wire_state[rom.out.0 as usize] = self.rom_devices[rom.device_slot].borrow().read().map(|w| w.unsigned()).unwrap_or(0);
                }
                wiring::ComponentWiring::MemorySystem(ms) => {
                    self.wire_state[ms.out.0 as usize] = self.ms_devices[ms.device_slot].borrow().read().map(|w| w.unsigned()).unwrap_or(0);
                }
                wiring::ComponentWiring::Serial(s) => {
                    self.wire_state[s.out.0 as usize] = <crate::device::Serial<D> as crate::device::MemoryDevice<A, D>>::read(&*self.serial_devices[s.device_slot].borrow()).map(|w| w.unsigned()).unwrap_or(0);
                }
                _ => {}
            }
        }

        eval_logic(&mut self.wire_state, &self.wiring.component_wiring);
    }
}

fn eval_logic(ws: &mut [u64], component_wiring: &[wiring::ComponentWiring]) {
    for comp in component_wiring {
        match comp {
            wiring::ComponentWiring::Nand(nand) => {
                let a = read_bit(ws, nand.a);
                let b = read_bit(ws, nand.b);
                write_bit(ws, nand.out, !(a & b));
            }
            wiring::ComponentWiring::Mux(mux) => {
                let sel = read_bit(ws, mux.sel);
                let src =
                    if !sel {
                        eval_logic(ws, &mux.branch0);
                        mux.a0
                    } else {
                        eval_logic(ws, &mux.branch1);
                        mux.a1
                    };
                ws[mux.out.0 as usize] = ws[src.0 as usize];
            }
            wiring::ComponentWiring::And(and) => {
                let a = read_bit(ws, and.a);
                let b = read_bit(ws, and.b);
                write_bit(ws, and.out, a & b);
            }
            wiring::ComponentWiring::Adder(add) => {
                let a = read_bit(ws, add.a) as u64;
                let b = read_bit(ws, add.b) as u64;
                let c = read_bit(ws, add.c) as u64;
                let total = a + b + c;  // 0..3
                write_bit(ws, add.sum,   total & 1 != 0);
                write_bit(ws, add.carry, total & 2 != 0);
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
pub enum BusResident<A: Nat + Storable, D: Nat + Storable> {
    RAM(RAMHandle<A, D>),
    ROM(ROMHandle<A, D>),
    Serial(SerialHandle<D>),
}

/// A clonable handle to a RAM instance (standalone or a region within a MemorySystem).
///
/// `base` is the region's base address in the memory map (0 for standalone RAM).
#[derive(Clone)]
pub struct RAMHandle<A: Nat + Storable, D: Nat + Storable> {
    pub base: usize,
    inner: Rc<RefCell<crate::device::RAM<A, D>>>,
}

impl<A: Nat + Storable, D: Nat + Storable> RAMHandle<A, D> {
    pub fn peek(&self, addr: u64) -> Word<D>             { self.inner.borrow().peek(Word::new(addr)).unwrap_or(Word::new(0)) }
    pub fn poke(&self, addr: u64, val: Word<D>)          { let _ = self.inner.borrow_mut().poke(Word::new(addr), val); }
    pub fn size(&self) -> usize                           { self.inner.borrow().size }
}

/// A clonable handle to a ROM instance in the simulated circuit.
#[derive(Clone)]
pub struct ROMHandle<A: Nat + Storable, D: Nat + Storable> {
    inner: Rc<RefCell<crate::device::ROM<A, D>>>,
}

impl<A: Nat + Storable, D: Nat + Storable> ROMHandle<A, D> {
    pub fn flash(&self, data: Vec<Word<D>>) {
        let _ = self.inner.borrow_mut().flash(data.into_boxed_slice());
    }
    pub fn size(&self) -> usize { self.inner.borrow().size }
}

/// A clonable handle to a Serial I/O device in the simulated circuit.
#[derive(Clone)]
pub struct SerialHandle<D: Nat + Storable> {
    inner: Rc<RefCell<crate::device::Serial<D>>>,
}

impl<D: Nat + Storable> SerialHandle<D> {
    /// Push a value from the outside world for the chip to read.
    pub fn push(&self, val: Word<D>)   { self.inner.borrow_mut().push(val); }
    /// Pull the last value written by the chip.
    pub fn pull(&self) -> Word<D>      { self.inner.borrow().pull() }
    /// Check whether the chip wrote during the last cycle.
    pub fn was_written(&self) -> bool { self.inner.borrow().was_written() }
    /// Clear the written flag.
    pub fn clear(&self)            { self.inner.borrow_mut().clear(); }
}
