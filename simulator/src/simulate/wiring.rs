use std::collections::HashMap;
use std::rc::Rc;

use crate::component::{Nand, Register16, RAM16, ROM16, MemorySystem16};
use crate::declare::{BusRef, Reflect};

/// Arbitrary (ptr) value identifying a wire's storage location. Used only during synthesis.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub(super) struct WireID(usize);

impl From<&BusRef> for WireID {
    fn from(busref: &BusRef) -> Self {
        WireID(Rc::as_ptr(&busref.id) as usize)
    }
}

/// Index of a wire's storage slot within a flat buffer.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub(super) struct WireIndex(pub(super) u32);

pub(super) type Indexes = HashMap<WireID, WireIndex>;

pub(super) enum ComponentWiring {
    Nand(NandWiring),
    ParallelNand(ParallelNandWiring),
    Register(RegisterWiring),
    ROM(ROMWiring),
    RAM(RAMWiring),
    MemorySystem(MemorySystemWiring),
    /// Note: output wiring for consts is not needed during evaluation because the bits are
    /// never updated.
    Const,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub(super) struct BitRef { pub(super) id: WireIndex, pub(super) offset: u8 }
impl BitRef {
    pub(super) fn new(b: &BusRef, ix: &Indexes) -> Self { BitRef { id: ix[&WireID::from(b)], offset: b.offset as u8 } }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub(super) struct WireRef { pub(super) id: WireIndex, pub(super) offset: u8, pub(super) width: u8 }
impl WireRef {
    pub(super) fn new(b: &BusRef, ix: &Indexes) -> Self { WireRef { id: ix[&WireID::from(b)], offset: b.offset as u8, width: b.width as u8 } }
}

pub(super) struct NandWiring { pub(super) a: BitRef, pub(super) b: BitRef, pub(super) out: BitRef }
impl NandWiring {
    pub(super) fn new(nand: &Nand, ix: &Indexes) -> Self {
        let intf = nand.reflect();
        Self {
            a:   BitRef::new(&intf.inputs["a"], ix),
            b:   BitRef::new(&intf.inputs["b"], ix),
            out: BitRef::new(&intf.outputs["out"], ix),
        }
    }
}

/// N parallel single-bit nands: out[i] = !(a[i] & b[i]). When multiple nands can be packed into the
/// same input/output locations, this is more efficient.
pub(super) struct ParallelNandWiring { pub(super) a: WireRef, pub(super) b: WireRef, pub(super) out: WireRef }
impl ParallelNandWiring {
    pub(super) fn new(a: WireRef, b: WireRef, out: WireRef) -> Self { Self { a, b, out } }
}

pub(super) struct RegisterWiring { pub(super) write: BitRef, pub(super) data_in: WireRef, pub(super) data_out: WireIndex }
impl RegisterWiring {
    pub(super) fn new(reg: &Register16, ix: &Indexes) -> Self {
        let intf = reg.reflect();
        Self {
            write:    BitRef::new(&intf.inputs["write"], ix),
            data_in:  WireRef::new(&intf.inputs["data_in"], ix),
            data_out: ix[&WireID::from(&intf.outputs["data_out"])],
        }
    }
}

pub(super) struct ROMWiring { pub(super) device_slot: usize, pub(super) out: WireRef, pub(super) addr: WireRef }
impl ROMWiring {
    pub(super) fn new(rom: &ROM16, slot: usize, ix: &Indexes) -> Self {
        let intf = rom.reflect();
        Self {
            device_slot: slot,
            out:  WireRef::new(&intf.outputs["out"], ix),
            addr: WireRef::new(&intf.inputs["addr"], ix),
        }
    }
}

pub(super) struct RAMWiring { pub(super) device_slot: usize, pub(super) out: WireRef, pub(super) addr: WireRef, pub(super) write: BitRef, pub(super) data_in: WireRef }
impl RAMWiring {
    pub(super) fn new(ram: &RAM16, slot: usize, ix: &Indexes) -> Self {
        let intf = ram.reflect();
        Self {
            device_slot: slot,
            out:     WireRef::new(&intf.outputs["data_out"], ix),
            addr:    WireRef::new(&intf.inputs["addr"], ix),
            write:   BitRef::new(&intf.inputs["write"], ix),
            data_in: WireRef::new(&intf.inputs["data_in"], ix),
        }
    }
}

pub(super) struct MemorySystemWiring { pub(super) device_slot: usize, pub(super) out: WireRef, pub(super) addr: WireRef, pub(super) write: BitRef, pub(super) data_in: WireRef }
impl MemorySystemWiring {
    pub(super) fn new(ms: &MemorySystem16, slot: usize, ix: &Indexes) -> Self {
        let intf = ms.reflect();
        Self {
            device_slot: slot,
            out:     WireRef::new(&intf.outputs["data_out"], ix),
            addr:    WireRef::new(&intf.inputs["addr"], ix),
            write:   BitRef::new(&intf.inputs["write"], ix),
            data_in: WireRef::new(&intf.inputs["data_in"], ix),
        }
    }
}
