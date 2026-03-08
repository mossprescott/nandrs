use crate::project_03::{PC, flatten};
use simulator::declare::{Chip as _};
use simulator::simulate::synthesize;

#[test]
fn pc_behavior() {
    let chip = flatten(PC::chip());
    let state = synthesize(&chip);

    assert_eq!(state.get("out"), 0);
}
