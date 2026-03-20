use std::fs;

use clap::Parser;

use assignments::project_05::{self, Computer, Project05Component, find_rom, memory_system};
use assignments::project_06::{assemble, Program};
use simulator::{IC, flatten, print_graph, print_ic_graph};
use simulator::declare::Chip as _;
use simulator::simulate::{synthesize, initialize};
use simulator::word::Word16;

use computer::cli::Args;
use computer::disasm::disassemble;
use computer::run;

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
        println!("{}", print_graph(&computer));

        // let squashed = half_flatten(Computer::chip().into());
        // println!("{}", print_ic_graph(&squashed));
    }
    let chip = project_05::flatten(computer);

    let Program { instructions, symbols } = program;

    let wiring = synthesize(&chip, memory_system());
    if args.print {
        print!("{wiring}");
    }

    if args.no_exec {
        return;
    }

    let state = initialize(wiring);

    find_rom(&state).flash(instructions.iter().map(|&v| Word16::from(v)).collect());

    let fmt_instr = |pc: Word16| -> String {
        instructions.get(pc.unsigned() as usize)
            .map(|&i| disassemble(i))
            .unwrap_or("?".to_string())
    };

    run(&args, state, &symbols, &fmt_instr);
}

/// Recursively expand high-level components (projects 3 and 5, for example), until only primitives
/// and simple logic are left (projects 1 and 2). Note that the result remains in the "project"
/// type, because it conveniently embeds the project 1 and 2 components, as well as the
/// Computational primitives.
fn half_flatten(chip: Project05Component) -> IC<Project05Component> {
    todo!()
    // flatten(chip, "simple", |c| None)
}