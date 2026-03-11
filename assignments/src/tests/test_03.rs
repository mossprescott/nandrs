use crate::project_03::{PC, flatten};
use simulator::declare::{Chip as _};
use simulator::simulate::{MemoryMap, synthesize};
use simulator::print_graph;

#[test]
fn pc_behavior() {
    let chip = PC::chip();

     // When it breaks, it's nice to see what it tried to do
    print!("{}", print_graph(&chip));

    let chip = flatten(chip);

    let no_ram = MemoryMap::new(vec![]);
    let mut state = synthesize(&chip, no_ram);

    assert_eq!(state.get("out"), 0);

    state.ticktock();

    assert_eq!(state.get("out"), 0); // No change: no flags set

    // "Normal" operation: inc is set and the value marches forward:

    state.set("inc", 1);

    assert_eq!(state.get("out"), 0); // No change: previous value still latched

    state.ticktock();
    assert_eq!(state.get("out"), 1);

    state.ticktock();
    assert_eq!(state.get("out"), 2);

    // Now hold the updated value:

    state.set("inc", 0);

    state.ticktock();

    assert_eq!(state.get("out"), 2);

    // Re-assert inc, but override it with a load:

    state.set("inc", 1);
    state.set("addr", 0x1234);
    state.set("load", 1);

    state.ticktock();
    assert_eq!(state.get("out"), 0x1234);

    state.ticktock();
    assert_eq!(state.get("out"), 0x1234);  // Load still in effect

    state.set("load", 0);
    state.ticktock();
    assert_eq!(state.get("out"), 0x1235);  // addr ignored now, back to inc

    // Pull the ejection switch:

    state.set("load", 1);  // Will be ignored while reset is asserted
    state.set("reset", 1);

    state.ticktock();
    assert_eq!(state.get("out"), 0);
}

#[test]
fn pc_optimal() {
    assert_eq!(flatten(PC::chip()).components.len(), 230);
}