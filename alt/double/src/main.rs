use clap::Parser;
use std::fs;

use assignments::project_05::memory_system;
use assignments::project_06::{assemble, Program};
use computer::cli::Args;
use computer::disasm::disassemble;
use double::computer::{Computer, DoubleComponent, find_roms, flatten as flatten_double, start};
use simulator::{Chip, IC, flatten, print_graph, print_ic_graph};
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

    let computer = Computer::chip();
    if args.print {
        println!("{}", print_graph(&computer));   // useless

        // let squashed = half_flatten(Computer::chip());
        // println!("{}", print_ic_graph(&squashed));
    }

    let chip = flatten_double(computer);

    // TODO: summarize the size of the chip in some way, for easier comparison with the standard impl.

    let Program { instructions, symbols } = program;

    let wiring = synthesize(&chip, memory_system());
    if args.print {
        print!("{wiring}");
    }

    if args.no_exec {
        return;
    }

    let mut state = initialize(wiring);

    // Each ROM gets its own copy of the same contents:
    let words: Vec<Word16> = instructions.iter().map(|&v| v.into()).collect();
    let (rom0, rom1) = find_roms(&state);
    rom0.flash(words.clone());
    rom1.flash(words);

    // Extra, mandatory init for the double-barreled PC:
    start(&mut state);

    let fmt_instr = |pc: Word16| -> String {
        instructions.get(pc.unsigned() as usize)
            .map(|&i| disassemble(i))
            .unwrap_or("?".to_string())
    };

    let fmt_instrs = |pc: Word16| -> String {
        let next = Word16::new(pc.unsigned() + 1);
        format!("{}, {}", fmt_instr(pc), fmt_instr(next))
    };

    computer::run(&args, state, &symbols, &fmt_instrs);
}

/// Recursively expand until only primitives and simple logic are left (projects 1 and 2).
fn half_flatten(chip: DoubleComponent) -> IC<DoubleComponent> {
    todo!()
    // flatten(chip, "simple", |c| None)
}