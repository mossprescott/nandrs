use crate::project_05::{MemorySystem, CPU, Computer, flatten, SCREEN_BASE, find_ram, find_screen, find_rom};
use crate::project_06::parse_statement;
use simulator::declare::Chip as _;
use simulator::simulate::synthesize;
use simulator::component::Computational;
use simulator::print_graph;

#[test]
fn memory_system_behavior() {
    let chip = MemorySystem::chip();

    // When it breaks, it's nice to see what it tried to do
    print!("{}", print_graph(&chip));

    let chip = flatten(chip);

    let mut state = synthesize(&chip);

    let ram    = find_ram(&state);
    let screen = find_screen(&state);

    state.set("addr", 0);
    assert_eq!(state.get("out"), 0);
    assert_eq!(ram.peek(0), 0);

    // Set up to write to the main RAM:
    state.set("data", 1234);
    state.set("load", 1);

    // Not latched yet:
    assert_eq!(state.get("out"), 0);
    assert_eq!(ram.peek(0), 0);

    // Now advance the clock:
    state.ticktock();
    assert_eq!(state.get("out"), 1234);
    assert_eq!(ram.peek(0), 1234);

    // Now write to the screen buffer:
    state.set("addr", SCREEN_BASE.into());
    state.set("data", 0x5555);

    state.ticktock();
    assert_eq!(state.get("out"), 0x5555);
    assert_eq!(screen.peek(0), 0x5555);  // Address is mapped to the base of the screen ram
    assert_eq!(ram.peek(0), 1234);  // Unaffected

    // Out-of-range address:
    state.set("addr", 0x8000);
    state.set("load", 0);
    assert_eq!(state.get("out"), 0);

    // Bad write; nothing explodes:
    state.set("data", 5678);
    state.set("load", 1);
    state.ticktock();
    assert_eq!(state.get("out"), 0);
}

#[test]
fn memory_system_optimal() {
    let components = flatten(MemorySystem::chip()).components;
    let nands = components.iter().filter(|c| matches!(c, Computational::Nand(_))).count();
    let rams  = components.iter().filter(|c| matches!(c, Computational::RAM(_))).count();
    assert_eq!(nands, 106);
    assert_eq!(rams,    2);
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
    // Note: values all available within the cycle
    assert_eq!(state.get("mem_write"), 1);
    assert_eq!(state.get("mem_out"), 1234);
    assert_eq!(state.get("mem_addr"), 256);
}

#[test]
fn cpu_optimal() {
    // PyNand has 1099 nands and 48 dffs
    // TODO: actually what?
    assert_eq!(flatten(CPU::chip()).components.len(), 947);
}

fn add_program() -> Vec<u64> {
    ["@2", "D=A", "@3", "D=D+A", "@1", "M=D"]
        .map(|op| instr(op).into())
        .to_vec()
}

#[test]
fn computer_add_behavior() {
    let chip = Computer::chip();

    // When it breaks, it's nice to see what it tried to do
    print!("{}", print_graph(&chip));

    let chip = flatten(chip);

    let mut state = synthesize(&chip);

    let rom = find_rom(&state);
    let ram = find_ram(&state);

    let pgm = add_program();
    rom.flash(pgm.clone());

    for _ in 0..pgm.len() { state.ticktock(); }

    assert_eq!(ram.peek(1), 5);
}

#[test]
fn computer_optimal() {
    let components = flatten(MemorySystem::chip()).components;
    let rams  = components.iter().filter(|c| matches!(c, Computational::RAM(_))).count();
    let roms  = components.iter().filter(|c| matches!(c, Computational::ROM(_))).count();
    let nands = components.iter().filter(|c| matches!(c, Computational::Nand(_))).count();
    assert_eq!(rams,    2);
    assert_eq!(roms,    1);
    assert_eq!(nands, 10000);
}
