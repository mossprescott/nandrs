use crate::project_05::{MemorySystem, CPU, Computer, flatten};
use crate::project_06::parse_statement;
use simulator::declare::Chip as _;
use simulator::simulate::{synthesize, BusResident};
use simulator::print_graph;

#[test]
fn memory_system_behavior() {
    let chip = MemorySystem::chip();

    // When it breaks, it's nice to see what it tried to do
    print!("{}", print_graph(&chip));

    let chip = flatten(chip);

    let mut state = synthesize(&chip);

    let find_ram = |size| state.bus_residents().iter()
        .find_map(|r| if let BusResident::RAM(h) = r { if h.size() == size { Some(h.clone()) } else { None } } else { None })
        .unwrap();
    let ram    = find_ram(16 * 1024); // 16KB main RAM
    let screen = find_ram( 8 * 1024); // 8KB screen buffer

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
    let SCREEN = 16384;
    state.set("addr", SCREEN);
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
    // TODO: count by type
    assert_eq!(flatten(MemorySystem::chip()).components.len(), 1);
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
