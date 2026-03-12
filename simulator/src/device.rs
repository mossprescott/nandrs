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

    data: Box<[u64]>,

    addr: Addr,
    valid: bool,
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

    data: Box<[u64]>,

    addr: Addr,
    next_addr: Addr,
    valid: bool,
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
