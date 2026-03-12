pub enum Error {
    AddressOutOfRange(usize),
    // NotReady,
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
}

impl MemoryDevice for ROM {
    fn set_addr(&mut self, addr: Addr) -> Result<(), Error> {
        if addr >= self.size {
            Err(Error::AddressOutOfRange(addr))
        } else {
            self.addr = addr;
            Ok(())
        }
    }

    /// Yawn.
    fn ticktock(&mut self) {}

    fn read(&self) -> Result<u64, Error> {
        Ok(self.data[self.addr])
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
}

impl MemoryDevice for RAM {
    fn set_addr(&mut self, addr: Addr) -> Result<(), Error> {
        if addr >= self.size {
            Err(Error::AddressOutOfRange(addr))
        } else {
            self.next_addr = addr;
            Ok(())
        }
    }

    fn ticktock(&mut self) {
        self.addr = self.next_addr;
    }

    fn read(&self) -> Result<u64, Error> {
        Ok(self.data[self.addr])
    }

    fn write(&mut self, word: Data) -> Result<(), Error> {
        self.data[self.addr] = word;
        Ok(())
    }
}

pub struct Overlay<T> {
    base: Addr,
    device: T,
}

/// Complete memory (sub)system by overlaying multiple devices at different locations in the address space.
pub struct MemorySystem<T> {
    devices: Vec<Overlay<T>>,
    active: Option<usize>,
}

impl<T: MemoryDevice> MemoryDevice for MemorySystem<T> {
    fn set_addr(&mut self, addr: Addr) -> Result<(), Error> {
        for (i, overlay) in self.devices.iter_mut().enumerate() {
            if addr >= overlay.base {
                if overlay.device.set_addr(addr - overlay.base).is_ok() {
                    self.active = Some(i);
                    return Ok(());
                }
            }
        }
        self.active = None;
        Err(Error::AddressOutOfRange(addr))
    }

    fn ticktock(&mut self) {
        for overlay in &mut self.devices {
            overlay.device.ticktock();
        }
    }

    fn read(&self) -> Result<Data, Error> {
        match self.active {
            Some(i) => self.devices[i].device.read(),
            None    => Err(Error::AddressOutOfRange(0)),
        }
    }

    fn write(&mut self, word: Data) -> Result<(), Error> {
        match self.active {
            Some(i) => self.devices[i].device.write(word),
            None    => Err(Error::AddressOutOfRange(0)),
        }
    }
}