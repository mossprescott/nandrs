use crate::project_05::{CPU, Computer, Decode, flatten, SCREEN_BASE, find_ram, find_screen, find_rom, memory_system};
use crate::project_06::parse_statement;
use simulator::declare::{Chip as _, IC};
use simulator::simulate::{simulate, ChipState, MemoryMap};
use simulator::component::{Computational, Computational16, MemorySystem16};
use simulator::print_graph;

/// Mostly this is testing the simulator's handling of the memory mapping we specified.
#[test]
fn memory_system_behavior() {
    let chip = flatten(MemorySystem16::chip());

    let mut state = simulate(&chip, memory_system());

    let ram    = find_ram(&state);
    let screen = find_screen(&state);

    state.set("addr", 0);
    state.ticktock();  // latch new address
    state.set("data_in", 1234);
    state.set("write", 1);

    // Now advance the clock:
    state.ticktock();
    assert_eq!(state.get("data_out"), 1234);
    assert_eq!(ram.peek(0), 1234);

    // Now write to the screen buffer:
    state.set("addr", SCREEN_BASE.into());
    state.ticktock();  // latch new address
    state.set("data_in", 0x5555);
    state.ticktock();

    assert_eq!(state.get("data_out"), 0x5555);
    assert_eq!(screen.peek(0), 0x5555);  // Address is mapped to the base of the screen ram
    assert_eq!(ram.peek(0), 1234);  // Unaffected

    // Out-of-range address; reads 0:
    state.set("addr", 0x8000);
    state.set("write", 0);
    state.ticktock();
    assert_eq!(state.get("data_out"), 0);

    // Bad write; nothing explodes:
    state.set("data_in", 5678);
    state.set("write", 1);
    state.ticktock();
    assert_eq!(state.get("data_out"), 0);
}

// #[test]
// fn memory_system_optimal() {
//     let components = flatten(MemorySystem::chip()).components;
//     let nands = components.iter().filter(|c| matches!(c, Computational::Nand(_))).count();
//     let rams  = components.iter().filter(|c| matches!(c, Computational::RAM(_))).count();
//     assert_eq!(nands, 106);
//     assert_eq!(rams,    2);
// }

fn simulate_loud(chip: IC<Computational16>, mmap: MemoryMap) -> ChipState {
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
    let mut state = simulate_loud(chip, no_ram);

    state.set("instr", instr("@1234").into());
    assert_eq!(state.get("is_c"), 0);

    state.set("instr", instr("D=0").into());
    assert_eq!(state.get("is_c"), 1);
    assert_eq!(state.get("write_d"), 1);
    assert_eq!(state.get("jmp_eq"), 0);
}

#[test]
fn decode_optimal() {
    let components = flatten(Decode::chip()).components;
    let nands = components.iter().filter(|c| matches!(c, Computational::Nand(_))).count();
    assert_eq!(nands, 0);
}

#[test]
fn cpu_behavior() {
    let chip = CPU::chip();

    // When it breaks, it's nice to see what it tried to do
    println!("{}", print_graph(&chip));

    let chip = flatten(chip);

    let no_ram = MemoryMap::new(vec![]);
    let mut state = simulate_loud(chip, no_ram);

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
    assert_eq!(state.get("mem_data_out"), 1234);
    assert_eq!(state.get("mem_addr"), 256);
}

#[test]
fn cpu_optimal() {
    // PyNand has 1099 nands and 48 dffs
    // TODO: actually what?
    let components = flatten(CPU::chip()).components;
    let nands = components.iter().filter(|c| matches!(c, Computational::Nand(_))).count();
    assert_eq!(nands, 931);
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

    let mut state = simulate(&chip, memory_system());

    let rom = find_rom(&state);
    let ram = find_ram(&state);

    let pgm = add_program();
    rom.flash(pgm.clone());

    for _ in 0..pgm.len() { state.ticktock(); }

    assert_eq!(state.get("pc"), 6);
    assert_eq!(ram.peek(1), 5);
}


fn max_program() -> Vec<u64> {
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
fn computer_max_behavior() {
    let chip = Computer::chip();

    // When it breaks, it's nice to see what it tried to do
    print!("{}", print_graph(&chip));

    let chip = flatten(chip);

    let mut state = simulate(&chip, memory_system());

    let rom = find_rom(&state);
    let ram = find_ram(&state);

    let pgm = max_program();
    rom.flash(pgm.clone());

    // Max in RAM[2]:
    ram.poke(1, 3);
    ram.poke(2, 5);

    // TODO: make the looping prologue automatic and factor this out
    while state.get("pc") <= (pgm.len()-2).try_into().unwrap() { state.ticktock(); }

    assert_eq!(ram.peek(3), 5);

    state.set("reset", 1);
    state.ticktock();
    state.set("reset", 0);

    // Max in RAM[1]:
    ram.poke(1, 23456);
    ram.poke(2, 12345);

    while state.get("pc") <= (pgm.len()-2).try_into().unwrap() { state.ticktock(); }

    assert_eq!(ram.peek(3), 23456);
}

#[test]
fn computer_indirect_write() {
    let chip = flatten(Computer::chip());
    let mut state = simulate(&chip, memory_system());

    let rom = find_rom(&state);
    let ram = find_ram(&state);

    // Store target address in R14, then write a value there
    let target: u64 = 100;
    ram.poke(14, target);

    let pgm: Vec<u64> = ["@14", "A=M", "M=1"]
        .map(|op| instr(op).into())
        .to_vec();
    rom.flash(pgm);

    for _ in 0..3 { state.ticktock(); }

    // assert_eq!(ram.peek(0), 1);  // Bug: writes here
    assert_eq!(ram.peek(target), 1);
}

#[test]
fn computer_indirect_jump() {
    let chip = flatten(Computer::chip());
    let mut state = simulate(&chip, memory_system());

    let rom = find_rom(&state);
    let ram = find_ram(&state);

    // Store target address in R14, then jump to it (as in a "call" or "return" sequence)
    let target: u64 = 100;
    ram.poke(14, target);

    let pgm: Vec<u64> = ["@14", "A=M", "JMP"]
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
    ram.poke(0, 257);
    ram.poke(256, 1234); // Not used, just simulating a value on the stack

    let pgm: Vec<u64> = [
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
    assert_eq!(ram.peek(0), 256);
    assert_eq!(ram.peek(5), 256);
}

#[test]
fn computer_optimal() {
    let components = flatten(Computer::chip()).components;
    let memsys = components.iter().filter(|c| matches!(c, Computational::MemorySystem(_))).count();
    let roms   = components.iter().filter(|c| matches!(c, Computational::ROM(_))).count();
    let nands  = components.iter().filter(|c| matches!(c, Computational::Nand(_))).count();
    assert_eq!(memsys,  1);
    assert_eq!(roms,    1);
    assert_eq!(nands,  959);
}
