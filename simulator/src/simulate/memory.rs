/// Descriptor for one contiguous RAM region in a memory map.
pub struct RAMMap {
    pub size: usize,
    pub base: usize,
}

pub struct ROMMap {
    pub size: usize,
    pub base: usize,
}

pub struct SerialMap {
    pub base: usize,
}

pub enum RegionMap {
    RAM(RAMMap),
    ROM(ROMMap),
    Serial(SerialMap),
}

/// Descriptor for the memory layout passed to [`super::synthesize`].
///
/// Specifies which regions exist and where they appear in the address space.
/// All actual data storage lives in device instances.
pub struct MemoryMap {
    pub regions: Vec<RegionMap>,
}

impl MemoryMap {
    pub fn empty() -> Self {
        MemoryMap { regions: vec![] }
    }
}
