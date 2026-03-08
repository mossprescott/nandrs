use crate::component::{Register16, Sequential16, RAM16, Computational16};
use crate::declare::{Chip as _, IC, Reflect as _};
use crate::simulate::synthesize;

#[test]
fn register_behavior() {
    let reg = Register16::chip();
    let chip = IC {
        name: reg.name().to_string(),
        intf: reg.reflect(),
        components: vec![Sequential16::Register(reg)],
    };
    let mut state = synthesize(&chip);

    assert_eq!(state.get("out"), 0);

    state.ticktock();
    assert_eq!(state.get("out"), 0); // load=0, no change

    state.set("data", 42);
    state.set("load", 1);
    assert_eq!(state.get("out"), 0); // still latched, no change

    state.ticktock();
    assert_eq!(state.get("out"), 42);

    state.set("data", 99);
    state.set("load", 0);

    state.ticktock();
    assert_eq!(state.get("out"), 42); // retained
}

#[test]
fn ram_behavior() {
    let ram = RAM16::chip();
    let chip = IC {
        name: ram.name().to_string(),
        intf: ram.reflect(),
        components: vec![Computational16::RAM(ram)],
    };
    let mut state = synthesize(&chip);

    assert_eq!(state.get("out"), 0);

    // Write 42 to address 5.
    state.set("addr", 5);
    state.set("data", 42);
    state.set("load", 1);
    state.ticktock();
    assert_eq!(state.get("out"), 42);

    // Write 99 to address 10.
    state.set("addr", 10);
    state.set("data", 99);
    state.ticktock();
    assert_eq!(state.get("out"), 99);

    // Read address 5 — other address unaffected.
    state.set("addr", 5);
    state.set("load", 0);
    state.ticktock();
    assert_eq!(state.get("out"), 42);

    // Unwritten address reads 0.
    state.set("addr", 0);
    state.ticktock();
    assert_eq!(state.get("out"), 0);
}

// TODO: test RAM latency
// TODO: test RAM limits (address out of bounds)