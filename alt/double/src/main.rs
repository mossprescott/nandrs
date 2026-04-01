use clap::Parser;
use std::fs;

use assignments::project_02::Project02Component;
use assignments::project_03::Project03Component;
use assignments::project_05::Project05Component;
use assignments::project_05::memory_system;
use assignments::project_06::{Program, assemble};
use computer::cli::Args;
use computer::disasm::disassemble;
use double::computer::{
    Computer, DoubleComponent, find_roms, flatten as flatten_double,
    flatten_for_simulation as flatten_double_sim,
};
use simulator::simulate::{initialize, synthesize};
use simulator::word::Word16;
use simulator::{Chip, Component, IC, flatten, print_ic_graph};

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
        let simple = simplify(Computer::chip());
        println!("{}", print_ic_graph(&simple));
    }

    let Program {
        instructions,
        symbols,
    } = program;

    let wiring = if args.precise {
        let chip = flatten_double(computer);
        synthesize(&chip, memory_system())
    } else {
        let chip = flatten_double_sim(computer);
        synthesize(&chip, memory_system())
    };
    if args.print {
        print!("{wiring}");
    }

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
        instructions
            .get(pc.unsigned() as usize)
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
fn simplify<C: Into<DoubleComponent>>(chip: C) -> IC<DoubleComponent> {
    flatten(chip.into(), "simple", &|c| match c {
        DoubleComponent::Project05(Project05Component::Project03(
            Project03Component::Project02(Project02Component::Project01(_)),
        )) => None,
        DoubleComponent::Project05(Project05Component::Project03(
            Project03Component::Project02(ref p2),
        )) => match p2 {
            Project02Component::ALU(_) => c.expand(),
            _ => None,
        },
        DoubleComponent::Project05(Project05Component::Project03(ref p3)) => match p3 {
            Project03Component::PC(_) => c.expand(),
            _ => None,
        },
        _ => c.expand(),
    })
}
