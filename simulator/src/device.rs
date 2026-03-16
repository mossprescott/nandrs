pub enum Error {
    AddressOutOfRange(usize),
    Unmapped,
    CannotWrite,
}

pub type Addr = usize;
pub type Data = u64;

/// Common interface for "bus-resident" devices that behave like memory.
///
/// - memory is single-ported; one address is in effect, and only that address can be read and/or
///   written during a given cycle.
/// - read/write devices can handle both a read and a write in the same cycle.
/// - each device determines how to handle latency.
/// - valid addresses are always between 0 and size-1
///
/// All addresses are usize, and all data is u64. It's up to the user to figure out how to pack the
/// actual simulated address and data word sizes into these.
pub trait MemoryDevice {
    /// Receive the address to be used for future reads and writes.
    fn set_addr(&mut self, addr: Addr) -> Result<(), Error>;

    /// Signals a chip/bus clock cycle boundary. Dependning on the device, the most-recently
    /// provided address might take effect at this moment.
    fn ticktock(&mut self);

    /// (Attempt to) read the current word.
    fn read(&self) -> Result<Data, Error>;

    /// (Attempt to) write the current word.
    fn write(&mut self, word: Data) -> Result<(), Error>;
}

/// Read-only storage with arbitrary size and no latency; just provide an address and get the data
/// immediately (within the current cycle.)
pub struct ROM {
    pub size: usize,

    data: Box<[Data]>,

    addr: Addr,
    valid: bool,
}

impl ROM {
    pub fn new(size: usize) -> Self {
        ROM { size, data: vec![0u64; size].into_boxed_slice(), addr: 0, valid: true }
    }

    /// For "external" users (not the simulation); overwrite the contents of the ROM
    /// during configuration.
    pub fn flash(&mut self, data: Box<[Data]>) -> Result<(), Error> {
        if data.len() > self.size {
            Err(Error::AddressOutOfRange(data.len()))
        } else {
            self.data = data;
            Ok(())
        }
    }
}

impl MemoryDevice for ROM {
    fn set_addr(&mut self, addr: Addr) -> Result<(), Error> {
        if addr >= self.size {
            self.valid = false;
            Err(Error::AddressOutOfRange(addr))
        } else {
            self.addr = addr;
            self.valid = true;
            Ok(())
        }
    }

    /// Yawn.
    fn ticktock(&mut self) {}

    fn read(&self) -> Result<u64, Error> {
        if self.valid { Ok(self.data[self.addr]) } else { Err(Error::AddressOutOfRange(0)) }
    }

    fn write(&mut self, _word: Data) -> Result<(), Error> {
        Err(Error::CannotWrite)
    }
}

/// Read-write storage with arbitrary size and one-cycle latency. When a new address arrives, it is
/// saved for use during the next cycle.
pub struct RAM {
    pub size: usize,

    data: Box<[Data]>,

    addr: Addr,
    next_addr: Addr,
    valid: bool,
}

impl RAM {
    pub fn new(size: usize) -> Self {
        RAM { size, data: vec![0u64; size].into_boxed_slice(), addr: 0, next_addr: 0, valid: false }
    }

    /// For "external" users (not the simulation); modify the contents of a location immediately.
    pub fn poke(&mut self, addr: Addr, word: Data) -> Result<(), Error> {
        if addr >= self.size {
            Err(Error::AddressOutOfRange(addr))
        } else {
            self.data[addr] = word;
            Ok(())
        }
    }

    /// For "external" users (not the simulation); inspect the contents of a location.
    pub fn peek(&self, addr: Addr) -> Result<Data, Error> {
        if addr >= self.size {
            Err(Error::AddressOutOfRange(addr))
        } else {
            Ok(self.data[addr])
        }
    }
}

impl MemoryDevice for RAM {
    fn set_addr(&mut self, addr: Addr) -> Result<(), Error> {
        if addr >= self.size {
            self.valid = false;
            Err(Error::AddressOutOfRange(addr))
        } else {
            self.next_addr = addr;
            self.valid = true;
            Ok(())
        }
    }

    fn ticktock(&mut self) {
        self.addr = self.next_addr;
    }

    fn read(&self) -> Result<u64, Error> {
        if self.valid { Ok(self.data[self.addr]) } else { Err(Error::AddressOutOfRange(0)) }
    }

    fn write(&mut self, word: Data) -> Result<(), Error> {
        if self.valid {
            self.data[self.addr] = word;
            Ok(())
        } else {
            Err(Error::AddressOutOfRange(0))
        }
    }
}

pub struct Overlay<T> {
    pub base: Addr,
    pub device: T,
}

/// Complete memory (sub)system by overlaying multiple devices at different locations in the address space.
///
/// On every operation, the devices are tried in order; the first device that can successfully
/// handle the operation is used. A device earlier in the list is effectively overlaid on top of
/// all later ones.
///
/// Each device handles its own address latching. `set_addr` is forwarded to every device so each
/// one can record whether the address falls within its range; `read`/`write` then return the first
/// `Ok` result.
pub struct MemorySystem<T> {
    pub devices: Vec<Overlay<T>>,
}

impl<T: MemoryDevice> MemoryDevice for MemorySystem<T> {
    /// Forward the address to every device (base-adjusted). Each device records whether it's valid.
    fn set_addr(&mut self, addr: Addr) -> Result<(), Error> {
        for overlay in &mut self.devices {
            let _ = overlay.device.set_addr(addr.checked_sub(overlay.base).unwrap_or(usize::MAX));
        }
        Ok(())
    }

    fn ticktock(&mut self) {
        for overlay in &mut self.devices {
            overlay.device.ticktock();
        }
    }

    /// Read from the first device that covers the current address.
    fn read(&self) -> Result<Data, Error> {
        for overlay in &self.devices {
            if let Ok(val) = overlay.device.read() {
                return Ok(val);
            }
        }
        Err(Error::Unmapped)
    }

    /// Write to the first device that covers the current address.
    fn write(&mut self, word: Data) -> Result<(), Error> {
        for overlay in &mut self.devices {
            if overlay.device.write(word).is_ok() {
                return Ok(());
            }
        }
        Err(Error::Unmapped)
    }
}

/// Single-word I/O device. The outside world pushes a value in; the chip can read it.
/// The chip can also write a value out.
///
/// Unlike RAM/ROM, there is no address — it's a single register in each direction.
pub struct Serial {
    /// Value available for the chip to read (set by the harness/outside world).
    read_val: Data,
    /// Value written by the chip (readable by the harness/outside world).
    write_val: Data,
    /// Whether the chip wrote this cycle.
    written: bool,
}

impl Serial {
    pub fn new() -> Self {
        Serial { read_val: 0, write_val: 0, written: false }
    }

    /// Push a value from the outside world for the chip to read.
    pub fn push(&mut self, val: Data) { self.read_val = val; }

    /// Pull the last value written by the chip (or 0 if nothing was written).
    pub fn pull(&self) -> Data { self.write_val }

    /// Check whether the chip wrote during the last cycle.
    pub fn was_written(&self) -> bool { self.written }

    /// Clear the written flag (call after pulling).
    pub fn clear(&mut self) { self.written = false; }
}

impl MemoryDevice for Serial {
    fn set_addr(&mut self, _addr: Addr) -> Result<(), Error> { Ok(()) }
    fn ticktock(&mut self) {}
    fn read(&self) -> Result<Data, Error> { Ok(self.read_val) }
    fn write(&mut self, word: Data) -> Result<(), Error> {
        self.write_val = word;
        self.written = true;
        Ok(())
    }
}

/// Allow `Rc<RefCell<RAM>>` to be used as a `MemoryDevice`, enabling shared ownership of a RAM
/// region (e.g. between a `MemorySystem` overlay and an external handle).
impl MemoryDevice for std::rc::Rc<std::cell::RefCell<RAM>> {
    fn set_addr(&mut self, addr: Addr) -> Result<(), Error>   { self.borrow_mut().set_addr(addr) }
    fn ticktock(&mut self)                                    { self.borrow_mut().ticktock(); }
    fn read(&self)                     -> Result<Data, Error> { self.borrow().read() }
    fn write(&mut self, word: Data)    -> Result<(), Error>   { self.borrow_mut().write(word) }
}
