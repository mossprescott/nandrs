use crate::project_05::{CPU, Computer, Decode, flatten, flatten_for_simulation, SCREEN_BASE, KEYBOARD, find_ram, find_screen, find_rom, find_keyboard, memory_system};
use crate::project_06::parse_statement;
use simulator::declare::{Chip as _, IC};
use simulator::simulate::{simulate, ChipState, MemoryMap};
use simulator::component::{Computational, Computational16, MemorySystem16, count_computational};
use simulator::nat::N16;
use simulator::{print_graph, print_ic_graph};
use simulator::word::Word16;

/// Mostly this is testing the simulator's handling of the memory mapping we specified.
#[test]
fn memory_system_behavior() {
    let chip = flatten(MemorySystem16::chip());

    let mut state = simulate(&chip, memory_system());

    let ram    = find_ram(&state);
    let screen = find_screen(&state);

    state.set("addr", 0u16.into());
    state.ticktock();  // latch new address
    state.set("data_in", 1234u16.into());
    state.set("write", true.into());

    // Now advance the clock:
    state.ticktock();
    assert_eq!(state.get("data_out"), 1234u16.into());
    assert_eq!(ram.peek(0), 1234u16.into());

    // Now write to the screen buffer:
    state.set("addr", SCREEN_BASE.into());
    state.ticktock();  // latch new address
    state.set("data_in", 0x5555u16.into());
    state.ticktock();

    assert_eq!(state.get("data_out"), 0x5555u16.into());
    assert_eq!(screen.peek(0), 0x5555u16.into());  // Address is mapped to the base of the screen ram
    assert_eq!(ram.peek(0), 1234u16.into());  // Unaffected

    // Out-of-range address; reads 0:
    state.set("addr", 0x8000u16.into());
    state.set("write", false.into());
    state.ticktock();
    assert_eq!(state.get("data_out"), 0u16.into());

    // Bad write; nothing explodes:
    state.set("data_in", 5678u16.into());
    state.set("write", true.into());
    state.ticktock();
    assert_eq!(state.get("data_out"), 0u16.into());
}

// #[test]
// fn memory_system_optimal() {
//     let components = flatten(MemorySystem::chip()).components;
//     let nands = components.iter().filter(|c| matches!(c, Computational::Nand(_))).count();
//     let rams  = components.iter().filter(|c| matches!(c, Computational::RAM(_))).count();
//     assert_eq!(nands, 106);
//     assert_eq!(rams,    2);
// }

fn simulate_loud(chip: &IC<Computational16>, mmap: MemoryMap) -> ChipState<N16, N16> {
    use simulator::simulate::{initialize, synthesize};

    let wiring = synthesize(&chip, mmap);

    // When it breaks, it's nice to see what the simulator translated it to
    println!("{}", wiring);
    initialize(wiring)
}

fn instr(stmt: &str) -> u16 {
    parse_statement(stmt).unwrap().raw().unwrap()
}

#[test]
fn decode_truth_table() {
     let chip = Decode::chip();

    // When it breaks, it's nice to see what it tried to do
    println!("{}", print_graph(&chip));

    let chip = flatten(chip);

    let no_ram = MemoryMap::new(vec![]);
    let mut state = simulate_loud(&chip, no_ram);

    state.set("instr", instr("@1234").into());
    assert_eq!(state.get("is_c"), false.into());

    state.set("instr", instr("D=0").into());
    assert_eq!(state.get("is_c"), true.into());
    assert_eq!(state.get("write_d"), true.into());
    assert_eq!(state.get("jmp_eq"), false.into());
}

#[test]
fn decode_strict_truth_table() {
     let chip = Decode::chip();

    // When it breaks, it's nice to see what it tried to do
    println!("{}", print_graph(&chip));

    let chip = flatten(chip);

    let no_ram = MemoryMap::new(vec![]);
    let mut state = simulate_loud(&chip, no_ram);

    // Every possible bit set:
    state.set("instr", instr("@0x7FFF").into());
    assert_eq!(state.get("is_c"), false.into());

    // All the control signals for the CPU are false:
    assert_eq!(state.get("write_a"), false.into());
    assert_eq!(state.get("write_d"), false.into());
    assert_eq!(state.get("write_m"), false.into());
    assert_eq!(state.get("jmp_lt"), false.into());
    assert_eq!(state.get("jmp_eq"), false.into());
    assert_eq!(state.get("jmp_gt"), false.into());

    // For the ALU, the Add operation is *not* selected.
    assert_eq!(state.get("f"), false.into());

}

#[test]
fn decode_optimal() {
    let chip = flatten(Decode::chip());
    assert_eq!(count_computational(&chip.components).nands, 17);
}

#[test]
fn cpu_behavior() {
    let chip = CPU::chip();

    // When it breaks, it's nice to see what it tried to do
    println!("{}", print_graph(&chip));

    let chip = flatten(chip);

    let no_ram = MemoryMap::new(vec![]);
    let mut state = simulate_loud(&chip, no_ram);

    // Load constant 1234 into A
    state.set("instr", instr("@1234").into());
    assert_eq!(state.get("mem_write"), false.into());
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
    assert_eq!(state.get("mem_write"), true.into());
    assert_eq!(state.get("mem_data_out"), 1234u16.into());
    assert_eq!(state.get("mem_addr"), 256u16.into());
}

#[test]
fn cpu_optimal() {
    // PyNand has 1099 nands and 48 dffs
    // TODO: actually what?
    let chip = flatten(CPU::chip());
    let counts = count_computational(&chip.components);
    assert_eq!(counts.nands, 1126);
    assert_eq!(counts.registers, 3);
}

fn add_program() -> Vec<Word16> {
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

    let mut state = simulate_loud(&chip, memory_system());

    let rom = find_rom(&state);
    let ram = find_ram(&state);

    let pgm = add_program();
    rom.flash(pgm.clone());

    for _ in 0..pgm.len() { state.ticktock(); }

    assert_eq!(state.get("pc"), 6u16.into());
    assert_eq!(ram.peek(1), 5u16.into());
}


pub fn max_program() -> Vec<Word16> {
    [
        "@1",
        "D=M",
        "@2",
        "D=D-M",
        "@10",
        "D;JGT",
        "@2",
        "D=M",  //   D = RAM[2]
        "@12",
        "JMP",
        "@1",   // 10
        "D=M",  //   D = RAM[1]
        "@3",   // 12
        "M=D",  //   RAM[3] = D (max)
        "@14",  // 14
        "JMP",  //   infinite loop
    ]
        .map(|op| instr(op).into())
        .to_vec()
}

#[test]
pub fn computer_max_behavior() {
    let chip = Computer::chip();

    // When it breaks, it's nice to see what it tried to do
    println!("{}", print_graph(&chip));

    let flat = flatten(chip);

    println!("{}", print_ic_graph(&flat));

    let state = simulate(&flat, memory_system());

    let rom = find_rom(&state);

    let pgm = max_program();
    rom.flash(pgm.clone());

    test_computer_max_behavior(state, pgm.len() as u64);
}

#[test]
pub fn computer_max_behavior_fast() {
    let chip = Computer::chip();

    // When it breaks, it's nice to see what it tried to do
    println!("{}", print_graph(&chip));

    let flat = flatten_for_simulation(Computer::chip());

    println!("{}", print_ic_graph(&flat));

    let state = simulate(&flat, memory_system());

    let rom = find_rom(&state);

    let pgm = max_program();
    rom.flash(pgm.clone());

    test_computer_max_behavior(state, pgm.len() as u64);
}

/// Run the simulation in the presence of "max_program" (assumed to be in ROM already) and verify
/// the result.
pub fn test_computer_max_behavior(mut state: ChipState<N16, N16>, max_iter: u64) {
    let ram = find_ram(&state);

    // Max in RAM[2]:
    ram.poke(1, 3u16.into());
    ram.poke(2, 5u16.into());

    // TODO: make the looping prologue automatic and factor this out
    for _ in 0..max_iter {
        println!("PC: {}", state.get("pc"));
        state.ticktock();
        if state.get("pc").unsigned() > max_iter { break; }
    }

    assert_eq!(ram.peek(3), 5u16.into());

    state.set("reset", true.into());
    state.ticktock();
    state.set("reset", false.into());

    // Max in RAM[1]:
    ram.poke(1, 23456u16.into());
    ram.poke(2, 12345u16.into());

    for _ in 0..max_iter {
        state.ticktock();
        if state.get("pc").unsigned() > max_iter { break; }
    }

    assert_eq!(ram.peek(3), 23456u16.into());
}

#[test]
fn computer_indirect_write() {
    let chip = flatten(Computer::chip());
    let mut state = simulate(&chip, memory_system());

    let rom = find_rom(&state);
    let ram = find_ram(&state);

    // Store target address in R14, then write a value there
    let target: u64 = 100;
    ram.poke(14, (target as u16).into());

    let pgm: Vec<Word16> = ["@14", "A=M", "M=1"]
        .map(|op| instr(op).into())
        .to_vec();
    rom.flash(pgm);

    for _ in 0..3 { state.ticktock(); }

    // assert_eq!(ram.peek(0), 1);  // Bug: writes here
    assert_eq!(ram.peek(target), 1u16.into());
}

#[test]
fn computer_indirect_jump() {
    let chip = flatten(Computer::chip());
    let mut state = simulate(&chip, memory_system());

    let rom = find_rom(&state);
    let ram = find_ram(&state);

    // Store target address in R14, then jump to it (as in a "call" or "return" sequence)
    let target: Word16 = 100u16.into();
    ram.poke(14, target);

    let pgm: Vec<Word16> = ["@14", "A=M", "JMP"]
        .map(|op| instr(op).into())
        .to_vec();
    rom.flash(pgm);

    for _ in 0..3 { state.ticktock(); }

    assert_eq!(state.get("pc"), target);
}

#[test]
fn computer_stack_adjust() {
    let chip = flatten(Computer::chip());
    let mut state = simulate(&chip, memory_system());

    let rom = find_rom(&state);
    let ram = find_ram(&state);

    // stack: [1234]
    ram.poke(0, 257u16.into());
    ram.poke(256, 1234u16.into()); // Not used, just simulating a value on the stack

    let pgm: Vec<Word16> = [
        "@0",
        "AM=M-1",  // adjust the stack pointer; A and R0 (aka SP) both = 256 now
        "D=A",     // now save A to R5
        "@5",
        "M=D",
    ]
        .map(|op| instr(op).into())
        .to_vec();
    rom.flash(pgm.clone());

    for _ in 0..pgm.len() { state.ticktock(); }

    // stack: [] (SP = 256); R5 = 256
    assert_eq!(ram.peek(0), 256u16.into());
    assert_eq!(ram.peek(5), 256u16.into());
}

#[test]
fn computer_read_keyboard() {
    let chip = flatten(Computer::chip());
    let mut state = simulate(&chip, memory_system());

    let rom = find_rom(&state);
    let ram = find_ram(&state);
    let keyboard = find_keyboard(&state);

    let pgm: Vec<Word16> = [
        "@24576", // KEYBOARD
        "D=M",
        "@5",
        "M=D",
    ]
        .map(|op| instr(op).into())
        .to_vec();
    rom.flash(pgm.clone());

    keyboard.push(76u16.into());

    for _ in 0..pgm.len() { state.ticktock(); }

    assert_eq!(ram.peek(5), 76u16.into());
    assert_eq!(ram.peek(KEYBOARD.into()), 0u16.into());  // The actual RAM is unaffected (doesn't map this address anyway)
}

#[test]
fn computer_optimal() {
    let chip = flatten(Computer::chip());
    let counts = count_computational(&chip.components);
    assert_eq!(counts.nands,  1126);
    assert_eq!(counts.registers, 3);
    assert_eq!(counts.roms,      1);
    assert_eq!(counts.memory_systems, 1);
    assert_eq!(chip.components.len(), counts.nands + counts.buffers + counts.registers + counts.roms + counts.memory_systems);
}

/// Component counts when flattened for simulation (with native Adder/Mux).
#[test]
fn computer_graph_for_simulation() {
    use simulator::simulate::native::count_simulational;
    let chip = flatten_for_simulation(Computer::chip());
    let counts = count_simulational(&chip.components);
    assert_eq!(counts.primitive.nands,        168);
    assert_eq!(counts.primitive.registers,      3);
    assert_eq!(counts.primitive.roms,           1);
    assert_eq!(counts.primitive.memory_systems, 1);
    assert_eq!(counts.muxes,                   15);
    assert_eq!(counts.mux1s,                    1);
    assert_eq!(counts.adders,                  31);
}
