use crate::project_03::{PC, flatten};
use simulator::declare::{Chip as _};
use simulator::component::{Sequential, count_sequential};
use simulator::nat::N16;
use simulator::simulate::{MemoryMap, simulate};
use simulator::print_graph;

#[test]
fn pc_behavior() {
    let chip = PC::chip();

     // When it breaks, it's nice to see what it tried to do
    print!("{}", print_graph(&chip));

    let chip = flatten(chip);

    let no_ram = MemoryMap::new(vec![]);
    let mut state = simulate::<_, N16, N16>(&chip, no_ram);

    assert_eq!(state.get("out"), 0u16.into());

    state.ticktock();

    assert_eq!(state.get("out"), 0u16.into()); // No change: no flags set

    // "Normal" operation: inc is set and the value marches forward:

    state.set("inc", true.into());

    assert_eq!(state.get("out"), 0u16.into()); // No change: previous value still latched

    state.ticktock();
    assert_eq!(state.get("out"), 1u16.into());

    state.ticktock();
    assert_eq!(state.get("out"), 2u16.into());

    // Now hold the updated value:

    state.set("inc", false.into());

    state.ticktock();

    assert_eq!(state.get("out"), 2u16.into());

    // Re-assert inc, but override it with a load:

    state.set("inc", true.into());
    state.set("addr", 0x1234u16.into());
    state.set("load", true.into());

    state.ticktock();
    assert_eq!(state.get("out"), 0x1234u16.into());

    state.ticktock();
    assert_eq!(state.get("out"), 0x1234u16.into());  // Load still in effect

    state.set("load", false.into());
    state.ticktock();
    assert_eq!(state.get("out"), 0x1235u16.into());  // addr ignored now, back to inc

    // Pull the ejection switch:

    state.set("load", true.into());  // Will be ignored while reset is asserted
    state.set("reset", true.into());

    state.ticktock();
    assert_eq!(state.get("out"), 0u16.into());
}

#[test]
fn pc_optimal() {
    let chip = flatten(PC::chip());
    let counts = count_sequential(&chip.components);
    assert_eq!(counts.nands, 283);
    assert_eq!(counts.registers, 1);
}
