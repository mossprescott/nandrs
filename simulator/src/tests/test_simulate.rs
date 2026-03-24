use crate::component::{Computational16, RAM16, Register16, Sequential16, Serial16};
use crate::declare::{Chip as _, IC, Reflect as _};
use crate::nat::N16;
use crate::simulate::{BusResident, MemoryMap, simulate};

#[test]
fn register_behavior() {
    let reg = Register16::chip();
    let chip = IC {
        name: reg.name().to_string(),
        intf: reg.reflect(),
        components: vec![Sequential16::Register(reg)],
    };
    let mut state = simulate::<_, N16, N16>(&chip, MemoryMap::new(vec![]));

    assert_eq!(state.get("data_out"), 0u16.into());

    state.ticktock();
    assert_eq!(state.get("data_out"), 0u16.into()); // write=0, no change

    state.set("data_in", 42u16.into());
    state.set("write", true.into());
    assert_eq!(state.get("data_out"), 0u16.into()); // still latched, no change

    state.ticktock();
    assert_eq!(state.get("data_out"), 42u16.into());

    state.set("data_in", 99u16.into());
    state.set("write", false.into());

    state.ticktock();
    assert_eq!(state.get("data_out"), 42u16.into()); // retained
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
    let mut state = simulate(&chip, MemoryMap::new(vec![]));

    assert_eq!(state.get("data_out"), 0u16.into());

    // Write 42 to address 5.
    state.set("addr", 5u16.into());
    state.ticktock(); // latch the address
    state.set("data_in", 42u16.into());
    state.set("write", true.into());
    state.ticktock();

    state.set("write", false.into());
    state.ticktock(); // allow to latch before reading
    assert_eq!(state.get("data_out"), 42u16.into());

    // Write 99 to address 10.
    state.set("addr", 10u16.into());
    state.ticktock(); // latch the address
    state.set("data_in", 99u16.into());
    state.set("write", true.into());
    state.ticktock();

    state.set("write", false.into());
    assert_eq!(state.get("data_out"), 99u16.into());

    // Read address 5 — other address unaffected.
    state.set("addr", 5u16.into());
    state.ticktock(); // latch the address
    state.set("write", false.into());
    state.ticktock();
    assert_eq!(state.get("data_out"), 42u16.into());

    // Unwritten address reads 0.
    state.set("addr", 0u16.into());
    state.ticktock(); // latch the address
    assert_eq!(state.get("data_out"), 0u16.into());
}

// TODO: test RAM latency
// TODO: test RAM limits (address out of bounds)

/// Test reading and writing data via the Serial device.
#[test]
fn serial_behavior() {
    let serial = Serial16::chip();
    let chip = IC {
        name: serial.name().to_string(),
        intf: serial.reflect(),
        components: vec![Computational16::Serial(serial)],
    };
    let mut state = simulate(&chip, MemoryMap::new(vec![]));

    let handle = state
        .bus_residents()
        .iter()
        .find_map(|r| {
            if let BusResident::Serial(h) = r {
                Some(h.clone())
            } else {
                None
            }
        })
        .expect("no serial device");

    // Initially reads 0.
    assert_eq!(state.get("data_out"), 0u16.into());

    // Push a value from the outside world; chip sees it after ticktock.
    handle.push(1234u16.into());
    state.ticktock();
    assert_eq!(state.get("data_out"), 1234u16.into());

    // Chip writes back via data_in + write strobe.
    state.set("data_in", 5678u16.into());
    state.set("write", true.into());
    state.ticktock();
    assert_eq!(handle.pull(), 5678u16.into());
    assert!(handle.was_written());

    // Clear and verify.
    handle.clear();
    assert!(!handle.was_written());

    // Push a new value; visible after ticktock.
    handle.push(42u16.into());
    state.set("write", false.into());
    state.ticktock();
    assert_eq!(state.get("data_out"), 42u16.into());
    assert_eq!(handle.pull(), 5678u16.into()); // last chip write still available
}
