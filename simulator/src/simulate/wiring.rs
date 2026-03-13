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

/// Location of the storage for a single-bit wire, at a certain word index and bit offset within the word.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub(super) struct BitRef { pub(super) id: WireIndex, pub(super) offset: u8 }
impl BitRef {
    pub(super) fn new(b: &BusRef, ix: &Indexes) -> Self { BitRef { id: ix[&WireID::from(b)], offset: b.offset as u8 } }
}

/// Location of the storage for a multi-bit wire (aka a bus), at a certain word index. Now used only
/// for input/output wiring when sub-component chips are tested in isolation. If then.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub(super) struct WireRef { pub(super) id: WireIndex, pub(super) offset: u8, pub(super) width: u8 }
impl WireRef {
    pub(super) fn new(b: &BusRef, ix: &Indexes) -> Self { WireRef { id: ix[&WireID::from(b)], offset: b.offset as u8, width: b.width as u8 } }
}

/// A single nand, referring to completely arbitrary bits of the words where it input and outputs
/// are stored.
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
///
/// TODO: if we discover patterns where only a sub-range of bits can be handled in parallel, we'll
/// add back offset/width fields *here*, applying to all three input/outputs, with a single
/// mask/shift.
pub(super) struct ParallelNandWiring { pub(super) a: WireIndex, pub(super) b: WireIndex, pub(super) out: WireIndex }
impl ParallelNandWiring {
    // TODO: as yet unclear how these will get built
    pub(super) fn new(a: WireIndex, b: WireIndex, out: WireIndex) -> Self { Self { a, b, out } }
}

pub(super) struct RegisterWiring { pub(super) write: BitRef, pub(super) data_in: WireIndex, pub(super) data_out: WireIndex }
impl RegisterWiring {
    pub(super) fn new(reg: &Register16, ix: &Indexes) -> Self {
        let intf = reg.reflect();
        Self {
            write:    BitRef::new(&intf.inputs["write"], ix),
            data_in:  ix[&WireID::from(&intf.inputs["data_in"])],
            data_out: ix[&WireID::from(&intf.outputs["data_out"])],
        }
    }
}

pub(super) struct ROMWiring { pub(super) device_slot: usize, pub(super) out: WireIndex, pub(super) addr: WireIndex }
impl ROMWiring {
    pub(super) fn new(rom: &ROM16, slot: usize, ix: &Indexes) -> Self {
        let intf = rom.reflect();
        Self {
            device_slot: slot,
            out:  ix[&WireID::from(&intf.outputs["out"])],
            addr: ix[&WireID::from(&intf.inputs["addr"])],
        }
    }
}

pub(super) struct RAMWiring { pub(super) device_slot: usize, pub(super) out: WireIndex, pub(super) addr: WireIndex, pub(super) write: BitRef, pub(super) data_in: WireIndex }
impl RAMWiring {
    pub(super) fn new(ram: &RAM16, slot: usize, ix: &Indexes) -> Self {
        let intf = ram.reflect();
        Self {
            device_slot: slot,
            out:     ix[&WireID::from(&intf.outputs["data_out"])],
            addr:    ix[&WireID::from(&intf.inputs["addr"])],
            write:   BitRef::new(&intf.inputs["write"], ix),
            data_in: ix[&WireID::from(&intf.inputs["data_in"])],
        }
    }
}

pub(super) struct MemorySystemWiring { pub(super) device_slot: usize, pub(super) out: WireIndex, pub(super) addr: WireIndex, pub(super) write: BitRef, pub(super) data_in: WireIndex }
impl MemorySystemWiring {
    pub(super) fn new(ms: &MemorySystem16, slot: usize, ix: &Indexes) -> Self {
        let intf = ms.reflect();
        Self {
            device_slot: slot,
            out:     ix[&WireID::from(&intf.outputs["data_out"])],
            addr:    ix[&WireID::from(&intf.inputs["addr"])],
            write:   BitRef::new(&intf.inputs["write"], ix),
            data_in: ix[&WireID::from(&intf.inputs["data_in"])],
        }
    }
}
