use std::fs;

use clap::Parser;

use assignments::project_01::{And, And16, Buffer, Mux, Mux16, Nand, Not, Not16, Or};
use assignments::project_02::{ALU, Add16, FullAdder, HalfAdder, Inc16, Nand16Way, Neg16, Zero16};
use assignments::project_03::PC;
use assignments::project_05::{
    self, CPU, Computer, Decode, Project05ComponentT, find_rom, memory_system,
};
use assignments::project_06::{Program, assemble};
use frunk::coproduct::CoprodInjector;
use frunk::hlist;
use simulator::component::{MemorySystem16, ROM16, Register16};
use simulator::declare::Chip as _;
use simulator::simulate::{initialize, synthesize};
use simulator::word::Word16;
use simulator::{Flat, IC, Reflect, flatten_g, print_ic_graph};

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
        let chip = project_05::flatten_t(computer);
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

macro_rules! preserve {
    ($c:expr) => {
        Flat::Done(vec![CoprodInjector::inject($c)])
    };
}

macro_rules! eliminate {
    ($c:expr) => {
        Flat::Continue($c.expand_t())
    };
}

/// Recursively expand high-level components (projects 3 and 5, essentially), until only primitives
/// and simple logic are left (projects 1 and 2, except the ALU).
fn simplify<C, Idx>(chip: C) -> IC<Project05ComponentT>
where
    C: Reflect,
    Project05ComponentT: CoprodInjector<C, Idx>,
{
    flatten_g::<C, Project05ComponentT, Idx, Project05ComponentT, _>(
        chip,
        "simple",
        hlist![
            // Project 01: stop
            |c: Nand| preserve!(c),
            |c: Buffer| preserve!(c),
            |c: Not| preserve!(c),
            |c: And| preserve!(c),
            |c: Or| preserve!(c),
            |c: Mux| preserve!(c),
            |c: Mux16| preserve!(c),
            |c: Not16| preserve!(c),
            |c: And16| preserve!(c),
            // Project 02: stop, except ALU which expands
            |c: HalfAdder| preserve!(c),
            |c: FullAdder| preserve!(c),
            |c: Inc16| preserve!(c),
            |c: Add16| preserve!(c),
            |c: Nand16Way| preserve!(c),
            |c: Zero16| preserve!(c),
            |c: Neg16| preserve!(c),
            |c: ALU| eliminate!(c),
            // Project 03+: stop registers, expand PC
            |c: Register16| preserve!(c),
            |c: PC| eliminate!(c),
            |c: ROM16| preserve!(c),
            |c: MemorySystem16| preserve!(c),
            // Project 05+: expand
            |c: Decode| eliminate!(c),
            |c: CPU| eliminate!(c),
            |c: Computer| eliminate!(c),
        ],
    )
}
