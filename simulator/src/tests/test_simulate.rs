use crate::component::{Register16, Sequential16, RAM16, Computational16};
use crate::declare::{Chip as _, IC, Reflect as _};
use crate::simulate::{synthesize, initialize, MemoryMap};

#[test]
fn register_behavior() {
    let reg = Register16::chip();
    let chip = IC {
        name: reg.name().to_string(),
        intf: reg.reflect(),
        components: vec![Sequential16::Register(reg)],
    };
    let mut state = initialize(synthesize(&chip, MemoryMap::new(vec![])));

    assert_eq!(state.get("data_out"), 0);

    state.ticktock();
    assert_eq!(state.get("data_out"), 0); // write=0, no change

    state.set("data_in", 42);
    state.set("write", 1);
    assert_eq!(state.get("data_out"), 0); // still latched, no change

    state.ticktock();
    assert_eq!(state.get("data_out"), 42);

    state.set("data_in", 99);
    state.set("write", 0);

    state.ticktock();
    assert_eq!(state.get("data_out"), 42); // retained
}

/// Test RAM's behavior vis-a-vis its inputs and outputs.
///
/// Note: to access a RAM in-situ in a larger chip, see ChipState::bus_residents().
#[test]
fn ram_behavior() {
    let ram = RAM16::chip(1024);
    let chip = IC {
        name: ram.name().to_string(),
        intf: ram.reflect(),
        components: vec![Computational16::RAM(ram)],
    };
    let mut state = initialize(synthesize(&chip, MemoryMap::new(vec![])));

    assert_eq!(state.get("data_out"), 0);

    // Write 42 to address 5.
    state.set("addr", 5);
    state.ticktock(); // latch the address
    state.set("data_in", 42);
    state.set("write", 1);
    state.ticktock();

    state.set("write", 0);
    state.ticktock(); // allow to latch before reading
    assert_eq!(state.get("data_out"), 42);

    // Write 99 to address 10.
    state.set("addr", 10);
    state.ticktock(); // latch the address
    state.set("data_in", 99);
    state.set("write", 1);
    state.ticktock();

    state.set("write", 0);
    assert_eq!(state.get("data_out"), 99);

    // Read address 5 — other address unaffected.
    state.set("addr", 5);
    state.ticktock(); // latch the address
    state.set("write", 0);
    state.ticktock();
    assert_eq!(state.get("data_out"), 42);

    // Unwritten address reads 0.
    state.set("addr", 0);
    state.ticktock(); // latch the address
    assert_eq!(state.get("data_out"), 0);
}

// TODO: test RAM latency
// TODO: test RAM limits (address out of bounds)