use clap::Parser;
use std::fs;

use assignments::project_05::find_rom;
use assignments::project_05::memory_system;
use assignments::project_06::{Program, assemble};
use computer::cli::Args;
use computer::disasm::disassemble;
use eight::computer::{
    Computer, flatten as flatten_eight, flatten_for_simulation as flatten_eight_sim,
};
use simulator::simulate::{initialize, synthesize};
use simulator::word::Word16;
use simulator::{Chip, print_ic_graph};

fn main() {
    let args = Args::parse();

    let src = fs::read_to_string(&args.path).unwrap_or_else(|e| {
        eprintln!("error reading {}: {e}", args.path);
        std::process::exit(1);
    });

    let program = assemble(&src).unwrap_or_else(|(e, line)| {
        eprintln!("assembly error on {:?}: {:?}", line, e);
        std::process::exit(1);
    });
    println!(
        "Loaded {} instructions from {}",
        program.instructions.len(),
        args.path
    );

    let computer = Computer::chip();
    if args.print {
        let simple = flatten_eight_sim(Computer::chip());
        println!("{}", print_ic_graph(&simple));
    }

    let Program {
        instructions,
        symbols,
    } = program;

    let wiring = if args.precise {
        let chip = flatten_eight(computer);
        synthesize(&chip, memory_system())
    } else {
        let chip = flatten_eight_sim(computer);
        synthesize(&chip, memory_system())
    };
    if args.print {
        print!("{wiring}");
    }

    if args.no_exec {
        return;
    }

    let state = initialize(wiring);

    let words: Vec<Word16> = instructions.iter().map(|&v| v.into()).collect();
    find_rom(&state).flash(words);

    let fmt_instr = |pc: Word16| -> String {
        instructions
            .get(pc.unsigned() as usize)
            .map(|&i| disassemble(i))
            .unwrap_or("?".to_string())
    };

    computer::run(&args, state, &symbols, &fmt_instr);
}
