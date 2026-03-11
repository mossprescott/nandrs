use std::collections::HashMap;
use std::env;
use std::fs;
use std::time::{Duration, Instant};

use minifb::{Key, Window, WindowOptions};

use assignments::project_05::{Computer, flatten, find_ram, find_rom, find_screen, memory_system};
use assignments::project_06::{assemble, Program};
use simulator::declare::Chip as _;
use simulator::simulate::{synthesize, RegionHandle};

const WIDTH: usize = 512;
const HEIGHT: usize = 256;
const FRAME_TIME: Duration = Duration::from_millis(16);

fn render_screen(screen: &RegionHandle, pixels: &mut [u32]) {
    for word_idx in 0..(WIDTH / 16 * HEIGHT) {
        let word = screen.peek(word_idx as u64) as u16;
        let row = word_idx / (WIDTH / 16);
        let col_word = word_idx % (WIDTH / 16);
        for bit in 0..16usize {
            let pixel_idx = row * WIDTH + col_word * 16 + bit;
            pixels[pixel_idx] = if (word >> bit) & 1 == 1 { 0x000000 } else { 0xFFFFFF };
        }
    }
}

fn disassemble(instr: u16) -> String {
    if instr & 0x8000 == 0 {
        return format!("@{}", instr & 0x7fff);
    }
    let a    = (instr >> 12) & 1;
    let comp = (instr >>  6) & 0x3f;
    let dest = (instr >>  3) & 0x7;
    let jump =  instr        & 0x7;

    let comp_str = match (a, comp) {
        (0, 0b101010) => "0",    (0, 0b111111) => "1",    (0, 0b111010) => "-1",
        (0, 0b001100) => "D",    (0, 0b110000) => "A",
        (0, 0b001101) => "!D",   (0, 0b110001) => "!A",
        (0, 0b001111) => "-D",   (0, 0b110011) => "-A",
        (0, 0b011111) => "D+1",  (0, 0b110111) => "A+1",
        (0, 0b001110) => "D-1",  (0, 0b110010) => "A-1",
        (0, 0b000010) => "D+A",  (0, 0b010011) => "D-A",
        (0, 0b000111) => "A-D",  (0, 0b000000) => "D&A",  (0, 0b010101) => "D|A",
        (1, 0b110000) => "M",    (1, 0b110001) => "!M",   (1, 0b110011) => "-M",
        (1, 0b110111) => "M+1",  (1, 0b110010) => "M-1",
        (1, 0b000010) => "D+M",  (1, 0b010011) => "D-M",
        (1, 0b000111) => "M-D",  (1, 0b000000) => "D&M",  (1, 0b010101) => "D|M",
        _ => "?",
    };
    let dest_str = match dest {
        0b000 => "",     0b001 => "M=",   0b010 => "D=",   0b011 => "DM=",
        0b100 => "A=",   0b101 => "AM=",  0b110 => "AD=",  0b111 => "ADM=",
        _ => unreachable!(),
    };
    let jump_str = match jump {
        0b000 => "",      0b001 => ";JGT", 0b010 => ";JEQ", 0b011 => ";JGE",
        0b100 => ";JLT",  0b101 => ";JNE", 0b110 => ";JLE", 0b111 => ";JMP",
        _ => unreachable!(),
    };
    format!("{}{}{}", dest_str, comp_str, jump_str)
}

/// Translate the currently held key to a Hack keycode (0 = no key).
fn hack_keycode(window: &Window) -> u64 {
    let shift = window.is_key_down(Key::LeftShift) || window.is_key_down(Key::RightShift);

    let specials: &[(Key, u64)] = &[
        (Key::Enter,    128), (Key::Backspace, 129),
        (Key::Left,     130), (Key::Up,        131),
        (Key::Right,    132), (Key::Down,      133),
        (Key::Home,     134), (Key::End,       135),
        (Key::PageUp,   136), (Key::PageDown,  137),
        (Key::Insert,   138), (Key::Delete,    139),
        (Key::Escape,   140),
        (Key::F1,  141), (Key::F2,  142), (Key::F3,  143), (Key::F4,  144),
        (Key::F5,  145), (Key::F6,  146), (Key::F7,  147), (Key::F8,  148),
        (Key::F9,  149), (Key::F10, 150), (Key::F11, 151), (Key::F12, 152),
    ];
    for &(key, code) in specials {
        if window.is_key_down(key) { return code; }
    }

    if window.is_key_down(Key::Space) { return b' ' as u64; }

    let letters = [
        Key::A, Key::B, Key::C, Key::D, Key::E, Key::F, Key::G, Key::H,
        Key::I, Key::J, Key::K, Key::L, Key::M, Key::N, Key::O, Key::P,
        Key::Q, Key::R, Key::S, Key::T, Key::U, Key::V, Key::W, Key::X,
        Key::Y, Key::Z,
    ];
    for (i, &key) in letters.iter().enumerate() {
        if window.is_key_down(key) {
            return (if shift { b'A' } else { b'a' } as usize + i) as u64;
        }
    }

    let digits = [
        Key::Key0, Key::Key1, Key::Key2, Key::Key3, Key::Key4,
        Key::Key5, Key::Key6, Key::Key7, Key::Key8, Key::Key9,
    ];
    for (i, &key) in digits.iter().enumerate() {
        if window.is_key_down(key) {
            return (b'0' as usize + i) as u64;
        }
    }

    0
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let trace   = args.contains(&"--trace".to_string());
    let verbose = args.contains(&"--verbose".to_string());
    let path = args.iter().find(|a| !a.starts_with('-') && *a != &args[0])
        .expect("usage: computer [--trace] [--verbose] <rom-file>");

    let src = fs::read_to_string(path).unwrap_or_else(|e| {
        eprintln!("error reading {path}: {e}");
        std::process::exit(1);
    });

    let Program { instructions, symbols } = assemble(&src);

    let mut symbols_by_addr: HashMap<u16, Vec<String>> = HashMap::new();
    for (name, addr) in &symbols {
        symbols_by_addr.entry(*addr).or_default().push(name.clone());
    }

    println!("computer: loaded {} instructions from {path}", instructions.len());

    eprint!("Synthesizing...");
    let chip = flatten(Computer::chip());
    let mut state = synthesize(&chip, memory_system());
    eprintln!(" done.");

    find_rom(&state).flash(instructions.iter().map(|&v| v as u64).collect());

    let ram = find_ram(&state);
    let screen = find_screen(&state);
    let mut pixels = vec![0u32; WIDTH * HEIGHT];

    let mut window = Window::new(path, WIDTH, HEIGHT, WindowOptions::default())
        .expect("failed to create window");

    eprintln!("Running.");

    let mut cycle: u64 = 0;
    let mut interval_start = Instant::now();
    let mut interval_cycles: u64 = 0;

    let print_state = |pc: u16, cycle: u64| {
        let labels = symbols_by_addr.get(&pc).map(|v| format!(" [{}]", v.join(", "))).unwrap_or_default();
        println!("pc={pc}{labels}: (cycle {cycle})");
        println!("  SP: {}", ram.peek(0));
        let asm = instructions.get(pc as usize).map(|&i| disassemble(i)).unwrap_or("?".to_string());
        println!("  {asm}")
    };

    if trace || verbose {
        print_state(state.get("pc") as u16, cycle);
    }

    while window.is_open() {
        let frame_start = Instant::now();
        let mut batch: u64 = 0;
        loop {
            state.ticktock();
            cycle += 1;
            interval_cycles += 1;
            batch += 1;

            if trace || verbose {
                let pc = state.get("pc") as u16;
                let labels = symbols_by_addr.get(&pc).map(|v| format!(" [{}]", v.join(", "))).unwrap_or_default();
                if verbose || !labels.is_empty() {
                    print_state(pc, cycle);
                }
            }

            // Check the clock every 256 cycles to avoid calling it every iteration.
            if batch & 255 == 0 && frame_start.elapsed() >= FRAME_TIME {
                break;
            }
        }

        // TODO: inject hack_keycode into simulator once keyboard RAM support is added
        let _key = hack_keycode(&window);

        render_screen(&screen, &mut pixels);
        window.update_with_buffer(&pixels, WIDTH, HEIGHT).unwrap();

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
            let cycle_f = cycle as f64;
            let (tval, tsuffix) = if cycle_f >= 1_000_000.0 {
                (cycle_f / 1_000_000.0, "M")
            } else if cycle_f >= 1_000.0 {
                (cycle_f / 1_000.0, "K")
            } else {
                (cycle_f, "")
            };
            println!("cycles/s: {val:.1}{suffix} (total: {tval:.1}{tsuffix})");
            interval_start = Instant::now();
            interval_cycles = 0;
        }
    }
}
