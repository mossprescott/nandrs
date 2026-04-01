use crate::nat::Nat;
use crate::word::{Storable, Word};

/// Enable crude but sometimes helpful logging of _all_ writes; reads can't really be logged
/// usefully because at the moment the RAM _always_ "reads" from teh current address, even if the
/// value isn't used.
const DEBUG_MEMORY: bool = false;

pub enum Error {
    AddressOutOfRange(usize),
    Unmapped,
    CannotWrite,
}

/// Common interface for "bus-resident" devices that behave like memory.
///
/// - memory is single-ported; one address is in effect, and only that address can be read and/or
///   written during a given cycle.
/// - read/write devices can handle both a read and a write in the same cycle.
/// - each device determines how to handle latency.
/// - valid addresses are always between 0 and size-1
pub trait MemoryDevice<A: Nat + Storable, D: Nat + Storable> {
    /// Receive the address to be used for future reads and writes.
    fn set_addr(&mut self, addr: Word<A>) -> Result<(), Error>;

    /// Signals a chip/bus clock cycle boundary. Depending on the device, the most-recently
    /// provided address might take effect at this moment.
    fn ticktock(&mut self);

    /// (Attempt to) read the current word.
    fn read(&self) -> Result<Word<D>, Error>;

    /// (Attempt to) write the current word.
    fn write(&mut self, word: Word<D>) -> Result<(), Error>;
}

/// Read-only storage with arbitrary size and no latency; just provide an address and get the data
/// immediately (within the current cycle.)
pub struct ROM<A: Nat + Storable, D: Nat + Storable> {
    pub size: usize,

    data: Box<[Word<D>]>,

    addr: Word<A>,
    valid: bool,
}

impl<A: Nat + Storable, D: Nat + Storable> ROM<A, D> {
    pub fn new(size: usize) -> Self {
        ROM {
            size,
            data: vec![Word::new(0); size].into_boxed_slice(),
            addr: Word::new(0),
            valid: true,
        }
    }

    /// For "external" users (not the simulation); overwrite the contents of the ROM
    /// during configuration.
    pub fn flash(&mut self, data: Box<[Word<D>]>) -> Result<(), Error> {
        if data.len() > self.size {
            Err(Error::AddressOutOfRange(data.len()))
        } else {
            // Pad to full size; unflashed locations read as 0.
            let mut buf = vec![Word::new(0); self.size].into_boxed_slice();
            buf[..data.len()].copy_from_slice(&data);
            self.data = buf;
            Ok(())
        }
    }
}

impl<A: Nat + Storable, D: Nat + Storable> MemoryDevice<A, D> for ROM<A, D> {
    fn set_addr(&mut self, addr: Word<A>) -> Result<(), Error> {
        let a = addr.unsigned() as usize;
        if a >= self.size {
            self.valid = false;
            Err(Error::AddressOutOfRange(a))
        } else {
            self.addr = addr;
            self.valid = true;
            Ok(())
        }
    }

    /// Yawn.
    fn ticktock(&mut self) {}

    fn read(&self) -> Result<Word<D>, Error> {
        if self.valid {
            Ok(self.data[self.addr.unsigned() as usize])
        } else {
            Err(Error::AddressOutOfRange(0))
        }
    }

    fn write(&mut self, _word: Word<D>) -> Result<(), Error> {
        Err(Error::CannotWrite)
    }
}

/// Read-write storage with arbitrary size and one-cycle latency. When a new address arrives, it is
/// saved for use during the next cycle.
pub struct RAM<A: Nat + Storable, D: Nat + Storable> {
    pub size: usize,

    data: Box<[Word<D>]>,

    addr: Word<A>,
    next_addr: Word<A>,
    valid: bool,
}

impl<A: Nat + Storable, D: Nat + Storable> RAM<A, D> {
    pub fn new(size: usize) -> Self {
        RAM {
            size,
            data: vec![Word::new(0); size].into_boxed_slice(),
            addr: Word::new(0),
            next_addr: Word::new(0),
            valid: false,
        }
    }

    /// For "external" users (not the simulation); modify the contents of a location immediately.
    pub fn poke(&mut self, addr: Word<A>, word: Word<D>) -> Result<(), Error> {
        let a = addr.unsigned() as usize;
        if a >= self.size {
            Err(Error::AddressOutOfRange(a))
        } else {
            self.data[a] = word;
            Ok(())
        }
    }

    /// For "external" users (not the simulation); inspect the contents of a location.
    pub fn peek(&self, addr: Word<A>) -> Result<Word<D>, Error> {
        let a = addr.unsigned() as usize;
        if a >= self.size {
            Err(Error::AddressOutOfRange(a))
        } else {
            Ok(self.data[a])
        }
    }
}

impl<A: Nat + Storable, D: Nat + Storable> MemoryDevice<A, D> for RAM<A, D> {
    fn set_addr(&mut self, addr: Word<A>) -> Result<(), Error> {
        let a = addr.unsigned() as usize;
        if a >= self.size {
            self.valid = false;
            Err(Error::AddressOutOfRange(a))
        } else {
            if DEBUG_MEMORY {
                println!("RAM set_addr({}); extent: {}", a, self.size);
            }
            self.next_addr = addr;
            self.valid = true;
            Ok(())
        }
    }

    fn ticktock(&mut self) {
        self.addr = self.next_addr;
    }

    fn read(&self) -> Result<Word<D>, Error> {
        if self.valid {
            Ok(self.data[self.addr.unsigned() as usize])
        } else {
            Err(Error::AddressOutOfRange(0))
        }
    }

    fn write(&mut self, word: Word<D>) -> Result<(), Error> {
        if DEBUG_MEMORY {
            println!("RAM write({}) @ addr={}; extent: {}", word, self.addr, self.size);
        }
        if self.valid {
            self.data[self.addr.unsigned() as usize] = word;
            Ok(())
        } else {
            Err(Error::AddressOutOfRange(0))
        }
    }
}

pub struct Overlay<A: Nat + Storable, T> {
    pub base: Word<A>,
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
pub struct MemorySystem<A: Nat + Storable, T> {
    pub devices: Vec<Overlay<A, T>>,
}

impl<A: Nat + Storable, D: Nat + Storable, T: MemoryDevice<A, D>> MemoryDevice<A, D>
    for MemorySystem<A, T>
{
    /// Forward the address to every device (base-adjusted). Each device records whether it's valid.
    fn set_addr(&mut self, addr: Word<A>) -> Result<(), Error> {
        for overlay in &mut self.devices {
            if addr.unsigned() >= overlay.base.unsigned() {
                let offset = addr.unsigned() - overlay.base.unsigned();
                let _ = overlay.device.set_addr(Word::new(offset));
            } else {
                // Address is below this region; invalidate without latching a bogus address.
                let _ = overlay.device.set_addr(Word::new(u64::MAX));
            }
        }
        Ok(())
    }

    fn ticktock(&mut self) {
        for overlay in &mut self.devices {
            overlay.device.ticktock();
        }
    }

    /// Read from the first device that covers the current address.
    fn read(&self) -> Result<Word<D>, Error> {
        for overlay in &self.devices {
            if let Ok(val) = overlay.device.read() {
                return Ok(val);
            }
        }
        Err(Error::Unmapped)
    }

    /// Write to the first device that covers the current address.
    fn write(&mut self, word: Word<D>) -> Result<(), Error> {
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
pub struct Serial<D: Nat + Storable> {
    /// Value available for the chip to read (set by the harness/outside world).
    read_val: Word<D>,
    /// Value written by the chip (readable by the harness/outside world).
    write_val: Word<D>,
    /// Whether the chip wrote this cycle.
    written: bool,
}

impl<D: Nat + Storable> Serial<D> {
    pub fn new() -> Self {
        Serial {
            read_val: Word::new(0),
            write_val: Word::new(0),
            written: false,
        }
    }

    /// Push a value from the outside world for the chip to read.
    pub fn push(&mut self, val: Word<D>) {
        self.read_val = val;
    }

    /// Pull the last value written by the chip (or 0 if nothing was written).
    pub fn pull(&self) -> Word<D> {
        self.write_val
    }

    /// Check whether the chip wrote during the last cycle.
    pub fn was_written(&self) -> bool {
        self.written
    }

    /// Clear the written flag (call after pulling).
    pub fn clear(&mut self) {
        self.written = false;
    }
}

impl<A: Nat + Storable, D: Nat + Storable> MemoryDevice<A, D> for Serial<D> {
    fn set_addr(&mut self, _addr: Word<A>) -> Result<(), Error> {
        Ok(())
    }

    fn ticktock(&mut self) {}

    fn read(&self) -> Result<Word<D>, Error> {
        Ok(self.read_val)
    }

    fn write(&mut self, word: Word<D>) -> Result<(), Error> {
        self.write_val = word;
        self.written = true;
        Ok(())
    }
}
