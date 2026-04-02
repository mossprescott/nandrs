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

/// Recursively expand until only primitives and simple logic are left (projects 1 and 2).
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
            |c: Nand| Flat::Done(vec![CoprodInjector::inject(c)]),
            |c: Buffer| Flat::Done(vec![CoprodInjector::inject(c)]),
            |c: Not| Flat::Done(vec![CoprodInjector::inject(c)]),
            |c: And| Flat::Done(vec![CoprodInjector::inject(c)]),
            |c: Or| Flat::Done(vec![CoprodInjector::inject(c)]),
            |c: Mux| Flat::Done(vec![CoprodInjector::inject(c)]),
            |c: Mux16| Flat::Done(vec![CoprodInjector::inject(c)]),
            |c: Not16| Flat::Done(vec![CoprodInjector::inject(c)]),
            |c: And16| Flat::Done(vec![CoprodInjector::inject(c)]),
            // Project 02: stop, except ALU which expands
            |c: HalfAdder| Flat::Done(vec![CoprodInjector::inject(c)]),
            |c: FullAdder| Flat::Done(vec![CoprodInjector::inject(c)]),
            |c: Inc16| Flat::Done(vec![CoprodInjector::inject(c)]),
            |c: Add16| Flat::Done(vec![CoprodInjector::inject(c)]),
            |c: Nand16Way| Flat::Done(vec![CoprodInjector::inject(c)]),
            |c: Zero16| Flat::Done(vec![CoprodInjector::inject(c)]),
            |c: Neg16| Flat::Done(vec![CoprodInjector::inject(c)]),
            |c: ALU| Flat::Continue(c.expand_t()),
            // Project 03+: stop registers, expand PC
            |c: Register16| Flat::Done(vec![CoprodInjector::inject(c)]),
            |c: PC| Flat::Continue(c.expand_t()),
            |c: ROM16| Flat::Done(vec![CoprodInjector::inject(c)]),
            |c: MemorySystem16| Flat::Done(vec![CoprodInjector::inject(c)]),
            // Project 05+: expand
            |c: Decode| Flat::Continue(c.expand_t()),
            |c: CPU| Flat::Continue(c.expand_t()),
            |c: Computer| Flat::Continue(c.expand_t()),
            |c: DoublePC| Flat::Continue(c.expand_t()),
            |c: Inc2| Flat::Continue(c.expand_t()),
        ],
    )
}
