use std::fs;

use clap::Parser;

use assignments::project_02::Project02Component;
use assignments::project_03::Project03Component;
use assignments::project_05::{self, Computer, Project05Component, find_rom, memory_system};
use assignments::project_06::{Program, assemble};
use simulator::declare::Chip as _;
use simulator::simulate::{initialize, synthesize};
use simulator::word::Word16;
use simulator::{Component, IC, flatten, print_ic_graph};

use computer::cli::Args;
use computer::disasm::disassemble;
use computer::run;

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
        let chip = project_05::flatten(computer);
        synthesize(&chip, memory_system())
    } else {
        let chip = project_05::flatten_for_simulation(computer);
        synthesize(&chip, memory_system())
    };
    if args.print {
        print!("{wiring}");
    }

    if args.no_exec {
        return;
    }

    let state = initialize(wiring);

    find_rom(&state).flash(instructions.iter().map(|&v| Word16::from(v)).collect());

    let fmt_instr = |pc: Word16| -> String {
        instructions
            .get(pc.unsigned() as usize)
            .map(|&i| disassemble(i))
            .unwrap_or("?".to_string())
    };

    run(&args, state, &symbols, &fmt_instr);
}

/// Recursively expand high-level components (projects 3 and 5, essentially), until only primitives
/// and simple logic are left (projects 1 and 2, except the ALU). Note that the result remains in
/// the "project_05" type, because it conveniently embeds the project 1 and 2 components, as well as
/// the Computational primitives.
fn simplify<C: Into<Project05Component>>(chip: C) -> IC<Project05Component> {
    flatten(chip.into(), "simple", &|c| match c {
        Project05Component::Project03(Project03Component::Project02(
            Project02Component::Project01(_),
        )) => None,
        Project05Component::Project03(Project03Component::Project02(ref p2)) => match p2 {
            Project02Component::ALU(_) => c.expand(),
            _ => None,
        },
        Project05Component::Project03(ref p3) => match p3 {
            Project03Component::PC(_) => c.expand(),
            _ => None,
        },
        _ => c.expand(),
    })
}
