use std::collections::HashMap;

use crate::declare::BusRef;

/// Arbitrary (ptr) value identifying a wire's storage location. Used only during synthesis.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub(super) struct WireID(usize);

impl From<&BusRef> for WireID {
    fn from(busref: &BusRef) -> Self {
        WireID(busref.id.0)
    }
}

/// Index of a wire's storage slot within a flat buffer.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub(super) struct WireIndex(pub(super) u32);

pub(super) type Indexes = HashMap<WireID, WireIndex>;


/// Location of the storage for a single-bit wire, at a certain word index and bit offset within the word.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub(super) struct BitRef { pub(super) id: WireIndex, pub(super) offset: u8 }

/// Location of the storage for a multi-bit wire (aka a bus), at a certain word index. Now used only
/// for input/output wiring when sub-component chips are tested in isolation. If then.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub(super) struct WireRef { pub(super) id: WireIndex, pub(super) offset: u8, pub(super) width: u8 }


/// Records connections involved in one step of evaluation. Could be called "Op", maybe?
#[derive(Clone)]
pub(super) enum ComponentWiring {
    // primitve:
    Nand(NandWiring),
    Mux(MuxWiring),
    Adder(AdderWiring),

    // external:
    Register(RegisterWiring),
    ROM(ROMWiring),
    RAM(RAMWiring),
    Serial(SerialWiring),
    MemorySystem(MemorySystemWiring),

    // synthetic:
    And(AndWiring),
    ParallelNand(ParallelNandWiring),
}

//
// Primitives:
//

/// A single nand, referring to completely arbitrary bits of the words where its inputs and output
/// are stored.
#[derive(Clone)]
pub(super) struct NandWiring {
    pub(super) a: BitRef,
    pub(super) b: BitRef,
    pub(super) out: BitRef
}

/// Select one result or another, as a primitive. Once the selector has been evaluated, only the
/// inputs needed for the "active" branch need to be evaluated.
#[derive(Clone)]
pub(super) struct MuxWiring {
    pub(super) sel: BitRef,
    pub(super) a0: WireIndex,
    pub(super) a1: WireIndex,
    pub(super) out: WireIndex,

    /// Wiring that needs to be updated in the case that sel == 0
    pub(super) branch0: Vec<ComponentWiring>,

    /// Wiring that needs to be updated in the case that sel == 1
    pub(super) branch1: Vec<ComponentWiring>,
}

/// Add a single bit-slice.
#[derive(Clone)]
pub(super) struct AdderWiring {
    pub(super) a: BitRef,
    pub(super) b: BitRef,
    pub(super) c: BitRef,
    pub(super) sum: BitRef,
    pub(super) carry: BitRef,
}


//
// External ("Bus-resident"):
//

#[derive(Clone)]
pub(super) struct RegisterWiring { pub(super) write: BitRef, pub(super) data_in: WireIndex, pub(super) data_out: WireIndex }

#[derive(Clone)]
pub(super) struct ROMWiring { pub(super) device_slot: usize, pub(super) out: WireIndex, pub(super) addr: WireIndex }

#[derive(Clone)]
pub(super) struct RAMWiring { pub(super) device_slot: usize, pub(super) out: WireIndex, pub(super) addr: WireIndex, pub(super) write: BitRef, pub(super) data_in: WireIndex }

#[derive(Clone)]
pub(super) struct MemorySystemWiring { pub(super) device_slot: usize, pub(super) out: WireIndex, pub(super) addr: WireIndex, pub(super) write: BitRef, pub(super) data_in: WireIndex }

#[derive(Clone)]
pub(super) struct SerialWiring { pub(super) device_slot: usize, pub(super) out: WireIndex, pub(super) write: BitRef, pub(super) data_in: WireIndex }



//
// "Synthetic" operations: result from coalescing multiple primitive operations which have related
// inputs and outputs; the host can handle lots of bits in a single operation when we detect those
// patterns.
//

/// Similar to NandWiring, but (un)inverting the result. This allows two steps to be collapsed
/// whenever this very common pattern occurs. Mostly because it's easier to read, but potentially
/// also because it's an easiear incremental step to coalescing many of them turn out to be
/// bit-parallel.
#[derive(Clone)]
pub(super) struct AndWiring {
    pub(super) a: BitRef,
    pub(super) b: BitRef,
    pub(super) out: BitRef
}

/// Bit-wise Nand of *all* bits, which are known to be aligned between all three wires.
///
/// No masking of inputs or output is needed (until proven otherwise.)
#[derive(Clone)]
pub(super) struct ParallelNandWiring {
    pub(super) a: WireIndex,
    pub(super) b: WireIndex,
    pub(super) out: WireIndex,
}

// TODO: ParallelAndWiring

//
// Constant values: come from `fixed()` inputs in the graph.
//

/// Used during initialization, bot evaluated each cycle.
pub(super) struct ConstWiring {
    pub(super) value: u64,
    pub(super) out: WireIndex,
}
