use crate::component::{Register16, Sequential16};
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
