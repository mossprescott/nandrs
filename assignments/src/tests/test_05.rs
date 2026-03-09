use crate::project_05::{MemorySystem, CPU, Computer, flatten};
use crate::project_06::parse_statement;
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

fn instr(stmt: &str) -> u16 {
    parse_statement(stmt).unwrap().raw().unwrap()
}

#[test]
fn cpu_behavior() {
    let chip = CPU::chip();

    // When it breaks, it's nice to see what it tried to do
    print!("{}", print_graph(&chip));

    let chip = flatten(chip);

    let mut state = synthesize(&chip);

    // Load constant 1234 into A
    state.set("instr", instr("@1234").into());
    assert_eq!(state.get("mem_write"), 0);
    state.ticktock();

    // Move it to D
    state.set("instr", instr("D=A").into());
    state.ticktock();

    // Load address 256 into A
    state.set("instr", instr("@256").into());
    state.ticktock();

    // "Write" to M (exposing the value)
    state.set("instr", instr("M=D").into());
    assert_eq!(state.get("mem_write"), 0);
    assert_eq!(state.get("mem_out"), 1234);
    assert_eq!(state.get("mem_addr"), 256);
    // Note: values all available within the cycle
}

#[test]
fn cpu_optimal() {
    // PyNand has 1099 nands and 48 dffs
    // TODO: actually what?
    assert_eq!(flatten(CPU::chip()).components.len(), 947);
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
