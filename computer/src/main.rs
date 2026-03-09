use std::collections::HashMap;
use std::env;
use std::fs;
use std::time::Instant;

use assignments::project_05::{Computer, flatten, find_rom};
use assignments::project_06::{assemble, Program};
use simulator::declare::Chip as _;
use simulator::simulate::synthesize;

fn main() {
    let args: Vec<String> = env::args().collect();
    let trace = args.contains(&"--trace".to_string());
    let path = args.iter().find(|a| !a.starts_with('-') && *a != &args[0])
        .expect("usage: computer [--trace] <rom-file>");

    let src = fs::read_to_string(path).unwrap_or_else(|e| {
        eprintln!("error reading {path}: {e}");
        std::process::exit(1);
    });

    let Program { instructions, symbols } = assemble(&src);

    // Invert: address → list of label names that point there.
    let mut symbols_by_addr: HashMap<u16, Vec<String>> = HashMap::new();
    for (name, addr) in &symbols {
        symbols_by_addr.entry(*addr).or_default().push(name.clone());
    }

    println!("computer: loaded {} instructions from {path}", instructions.len());

    let chip = flatten(Computer::chip());
    let mut state = synthesize(&chip);

    find_rom(&state).flash(instructions.into_iter().map(|v| v as u64).collect());

    let mut cycle: u64 = 0;
    let mut interval_start = Instant::now();
    let mut interval_cycles: u64 = 0;
    loop {
        state.ticktock();
        cycle += 1;
        interval_cycles += 1;

        let elapsed = interval_start.elapsed();
        if elapsed.as_secs() >= 1 {
            let cps = interval_cycles as f64 / elapsed.as_secs_f64();
            let (val, suffix) = if cps >= 1_000_000.0 {
                (cps / 1_000_000.0, "M")
            } else if cps >= 1_000.0 {
                (cps / 1_000.0, "K")
            } else {
                (cps, "")
            };
            println!("cycles/s: {val:.1}{suffix}");
            interval_start = Instant::now();
            interval_cycles = 0;
        }

        if trace {
            let pc = state.get("pc") as u16;
            if let Some(labels) = symbols_by_addr.get(&pc) {
                println!("@{pc} {} (cycle {cycle:>10})", labels.join(", "));
            }
        }
    }
}
