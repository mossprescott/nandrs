use clap::Parser;
use std::fs;

use assignments::project_01::{And, And16, Buffer, Mux, Mux16, Nand, Not, Not16, Or};
use assignments::project_02::{ALU, Add16, FullAdder, HalfAdder, Inc16, Nand16Way, Neg16, Zero16};
use assignments::project_03::PC;
use assignments::project_05::Decode;
use assignments::project_05::memory_system;
use assignments::project_06::{Program, assemble};
use computer::cli::Args;
use computer::disasm::disassemble;
use double::computer::{
    CPU, Computer, DoubleComponentT, DoublePC, Inc2, find_roms,
    flatten_for_simulation as flatten_double_sim, flatten_t as flatten_double,
};
use frunk::coproduct::CoprodInjector;
use frunk::hlist;
use simulator::component::{MemorySystem16, ROM16, Register16};
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

/// Channeling dfithian. This stuff is just hard to look at.
macro_rules! preserve {
    ($c:expr) => {
        Flat::Done(vec![CoprodInjector::inject($c)])
    };
}

/// Channeling dfithian. This stuff is just hard to look at.
macro_rules! eliminate {
    ($c:expr) => {
        Flat::Continue($c.expand_t())
    };
}

/// Recursively expand until only primitives and simple logic are left (projects 1 and 2).
///
/// TODO: figure out how to share the portion of this fold which is common with the project05
/// components, which is to say everything except fe defined here, which should all be expanded.
fn simplify<C, Idx>(chip: C) -> IC<DoubleComponentT>
where
    C: Reflect,
    DoubleComponentT: frunk::coproduct::CoprodInjector<C, Idx>,
{
    flatten_g::<C, DoubleComponentT, Idx, DoubleComponentT, _>(
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
            |c: DoublePC| eliminate!(c),
            |c: Inc2| eliminate!(c),
        ],
    )
}
