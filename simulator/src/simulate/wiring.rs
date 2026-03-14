use std::collections::HashMap;
use std::rc::Rc;

use crate::declare::BusRef;

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

/// Records connections involved in one step of evaluation. Could be called "Op", maybe?
pub(super) enum ComponentWiring {
    Nand(NandWiring),
    Mux(MuxWiring),
    Register(RegisterWiring),
    ROM(ROMWiring),
    RAM(RAMWiring),
    MemorySystem(MemorySystemWiring),
    // /// Note: output wiring for consts is not needed during evaluation because the bits are
    // /// never updated.
    // Const,
    // TODO: Not? And? Other re-constructed primitives to save ops?
}

/// Location of the storage for a single-bit wire, at a certain word index and bit offset within the word.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub(super) struct BitRef { pub(super) id: WireIndex, pub(super) offset: u8 }

/// Location of the storage for a multi-bit wire (aka a bus), at a certain word index. Now used only
/// for input/output wiring when sub-component chips are tested in isolation. If then.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub(super) struct WireRef { pub(super) id: WireIndex, pub(super) offset: u8, pub(super) width: u8 }

/// A single nand, referring to completely arbitrary bits of the words where it input and outputs
/// are stored.
pub(super) struct NandWiring { pub(super) a: BitRef, pub(super) b: BitRef, pub(super) out: BitRef }

pub(super) struct MuxWiring { pub(super) sel: BitRef, pub(super) a0: WireIndex, pub(super) a1: WireIndex, pub(super) out: WireIndex }

pub(super) struct RegisterWiring { pub(super) write: BitRef, pub(super) data_in: WireIndex, pub(super) data_out: WireIndex }

pub(super) struct ROMWiring { pub(super) device_slot: usize, pub(super) out: WireIndex, pub(super) addr: WireIndex }

pub(super) struct RAMWiring { pub(super) device_slot: usize, pub(super) out: WireIndex, pub(super) addr: WireIndex, pub(super) write: BitRef, pub(super) data_in: WireIndex }

pub(super) struct MemorySystemWiring { pub(super) device_slot: usize, pub(super) out: WireIndex, pub(super) addr: WireIndex, pub(super) write: BitRef, pub(super) data_in: WireIndex }


pub(super) struct ConstWiring {
    pub(super) value: u64,
    pub(super) out: WireIndex,
}
