use std::fs;

use clap::Parser;

use assignments::project_05::memory_system;
use assignments::project_06::{assemble, Program};
use computer::cli::Args;
use computer::disasm::disassemble;
use double::computer::{Computer, flatten, find_roms};
use simulator::Chip;
use simulator::simulate::{synthesize, initialize};
use simulator::word::Word16;

fn main() {
    let args = Args::parse();

    let src = fs::read_to_string(&args.path).unwrap_or_else(|e| {
        eprintln!("error reading {}: {e}", args.path);
        std::process::exit(1);
    });

    let program = assemble(&src);
    println!("Loaded {} instructions from {}", program.instructions.len(), args.path);

    let chip = flatten(Computer::chip());

    let Program { instructions, symbols } = program;

    let wiring = synthesize(&chip, memory_system());

    if args.no_exec {
        return;
    }

    let state = initialize(wiring);

    // Each ROM gets its own copy of the same contents:
    let words: Vec<Word16> = instructions.iter().map(|&v| v.into()).collect();
    let (rom0, rom1) = find_roms(&state);
    rom0.flash(words.clone());
    rom1.flash(words);

    let fmt_instr = |pc: Word16| -> String {
        instructions.get(pc.unsigned() as usize)
            .map(|&i| disassemble(i))
            .unwrap_or("?".to_string())
    };

    computer::run(&args, state, &symbols, &fmt_instr);
}
