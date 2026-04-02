use clap::Parser;
use std::fs;

use assignments::project_01::{And, Buffer, Mux, Nand, Not, Or};
use assignments::project_02::{FullAdder, HalfAdder};
use assignments::project_05::Decode;
use assignments::project_05::find_rom;
use assignments::project_05::memory_system;
use assignments::project_06::{Program, assemble};
use computer::cli::Args;
use computer::disasm::disassemble;
use eight::component::{Latch1, Latch8, Register8};
use eight::computer::{
    ALU, Add8, And8, CPU, Computer, EightComponentT, Inc8, Join, Mux8, Nand8Way, Neg8, Not8, PC,
    Split, Zero8, flatten_for_simulation as flatten_eight,
};
use frunk::coproduct::CoprodInjector;
use frunk::hlist;
use simulator::component::{MemorySystem16, ROM16};
use simulator::simulate::{initialize, synthesize};
use simulator::word::Word16;
use simulator::{Chip, Flat, IC, Reflect, flatten_g, print_ic_graph};

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

    let wiring = {
        let chip = flatten_eight(computer);
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

/// Channeling dfithian. This stuff is just hard to look at.
macro_rules! preserve {
    ($c:expr) => {
        Flat::Done(vec![CoprodInjector::inject($c)])
    };
}

/// Channeling dfithian. This stuff is just hard to look at.
macro_rules! eliminate {
    ($c:expr) => {
        Flat::Continue($c.expand())
    };
}

/// Recursively expand until only primitives and simple logic are left.
fn simplify<C, Idx>(chip: C) -> IC<EightComponentT>
where
    C: Reflect,
    EightComponentT: CoprodInjector<C, Idx>,
{
    flatten_g::<C, EightComponentT, Idx, EightComponentT, _>(
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
            // Project 02 equivalents: stop
            |c: HalfAdder| preserve!(c),
            |c: FullAdder| preserve!(c),
            |c: Mux8| preserve!(c),
            |c: Not8| preserve!(c),
            |c: And8| preserve!(c),
            |c: Inc8| preserve!(c),
            |c: Add8| preserve!(c),
            |c: Nand8Way| preserve!(c),
            |c: Zero8| preserve!(c),
            |c: Neg8| preserve!(c),
            |c: ALU| eliminate!(c),
            |c: Split| preserve!(c),
            |c: Join| preserve!(c),
            // Project 03+: stop registers, expand PC
            |c: Decode| preserve!(c),
            |c: Register8| preserve!(c),
            |c: Latch8| eliminate!(c),
            |c: Latch1| preserve!(c),
            |c: ROM16| preserve!(c),
            |c: MemorySystem16| preserve!(c),
            |c: PC| eliminate!(c),
            // Project 05+: expand
            |c: CPU| eliminate!(c),
            |c: Computer| eliminate!(c),
        ],
    )
}
