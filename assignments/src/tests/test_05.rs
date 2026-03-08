use crate::project_05::{MemorySystem, CPU, Computer, flatten};
use simulator::declare::Chip as _;
use simulator::simulate::synthesize;
use simulator::print_graph;

#[test]
fn memory_system_behavior() {
    todo!()
}

#[test]
fn memory_system_optimal() {
    // TODO: count by type
    assert_eq!(flatten(MemorySystem::chip()).components.len(), todo!());
}

#[test]
fn cpu_behavior() {
    todo!()
}

#[test]
fn cpu_optimal() {
    assert_eq!(flatten(CPU::chip()).components.len(), todo!());
}

#[test]
fn computer_behavior() {
    let chip = Computer::chip();

     // When it breaks, it's nice to see what it tried to do
    print!("{}", print_graph(&chip));

    let chip = flatten(chip);

    let mut state = synthesize(&chip);

    assert_eq!(state.get("out"), 0);
}

#[test]
fn computer_optimal() {
    assert_eq!(flatten(Computer::chip()).components.len(), todo!());
}
