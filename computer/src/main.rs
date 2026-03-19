use std::collections::HashMap;
use std::env;
use std::fs;
use std::time::{Duration, Instant};

use minifb::{Key, Window, WindowOptions};

use assignments::project_03::Project03Component;
use assignments::project_05::{Computer, Project05Component, flatten, find_ram, find_rom, find_screen, find_keyboard, memory_system};
use assignments::project_06::{assemble, Program};
use simulator::{IC, Reflect, Component as _};
use simulator::{print_graph, print_ic_graph};
use simulator::declare::Chip as _;
use simulator::simulate::{synthesize, initialize, RAMHandle};
use simulator::nat::N16;
use simulator::word::Word16;

const WIDTH: usize = 512;
const HEIGHT: usize = 256;
const BEZEL: usize = 20;
const FRAME_TIME: Duration = Duration::from_millis(1000/60);
const BEZEL_PNG: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/bezel.png");

fn render_screen(screen: &RAMHandle<N16, N16>, pixels: &mut [u32], scale: usize) {
    let win_width = (WIDTH + 2 * BEZEL) * scale;
    for word_idx in 0..(WIDTH / 16 * HEIGHT) {
        let word = screen.peek(word_idx as u64).unsigned() as u16;
        let row = word_idx / (WIDTH / 16);
        let col_word = word_idx % (WIDTH / 16);
        for bit in 0..16usize {
            let color = if (word >> bit) & 1 == 1 { 0x000000 } else { 0xFFFFFF };
            let px_x = BEZEL + col_word * 16 + bit;
            let px_y = BEZEL + row;
            for dy in 0..scale {
                for dx in 0..scale {
                    pixels[(px_y * scale + dy) * win_width + px_x * scale + dx] = color;
                }
            }
        }
    }
}

fn load_bezel(scale: usize) -> Vec<u32> {
    let file = std::fs::File::open(BEZEL_PNG)
        .unwrap_or_else(|e| panic!("cannot open {BEZEL_PNG}: {e}"));
    let decoder = png::Decoder::new(file);
    let mut reader = decoder.read_info().expect("png read_info");
    let mut buf = vec![0u8; reader.output_buffer_size()];
    let info = reader.next_frame(&mut buf).expect("png next_frame");
    let bytes = &buf[..info.buffer_size()];
    let src_w = info.width as usize;
    let src_h = info.height as usize;
    let bpp = match info.color_type {
        png::ColorType::Rgb  => 3,
        png::ColorType::Rgba => 4,
        _ => panic!("unsupported bezel PNG color type"),
    };
    let dst_w = src_w * scale;
    let dst_h = src_h * scale;
    let mut out = vec![0u32; dst_w * dst_h];
    for sy in 0..src_h {
        for sx in 0..src_w {
            let i = (sy * src_w + sx) * bpp;
            let c = ((bytes[i] as u32) << 16) | ((bytes[i+1] as u32) << 8) | (bytes[i+2] as u32);
            for dy in 0..scale {
                for dx in 0..scale {
                    out[(sy * scale + dy) * dst_w + sx * scale + dx] = c;
                }
            }
        }
    }
    out
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

fn fmt_commas(n: u64) -> String {
    let s = n.to_string();
    let mut out = String::new();
    for (i, c) in s.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 { out.push(','); }
        out.push(c);
    }
    out.chars().rev().collect()
}

/// Recursively expand high-level components (projects 3 and 5), until only primitives and simple
/// logic are left (projects 1 and 2).
pub fn half_flatten<C: Reflect + Into<Project05Component>>(chip: C) -> IC<Project05Component> {
    fn go(comp: Project05Component) -> Vec<Project05Component> {
        // Stop at Project02: don't expand ALU, adders, etc. into Nands.
        if let Project05Component::Project03(Project03Component::Project02(_)) = &comp {
            vec![comp]
        }
        else {
            match comp.expand() {
                None => vec![comp],
                Some(ic) => ic.components.into_iter().flat_map(go).collect(),
            }
        }
    }
    IC {
        name: format!("{} (half-flat)", chip.name()),
        intf: chip.reflect(),
        components: go(chip.into()),
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let trace   = args.contains(&"--trace".to_string());
    let verbose = args.contains(&"--verbose".to_string());
    let print   = args.contains(&"--print".to_string());
    let no_exec = args.contains(&"--no-exec".to_string());
    let scale   = if args.contains(&"--2x".to_string()) { 2 } else { 1 };
    let path = args.iter().find(|a| !a.starts_with('-') && *a != &args[0])
        .expect("usage: computer [--trace] [--verbose] [--print] [--no-exec] [--2x] <rom-file>");

    let src = fs::read_to_string(path).unwrap_or_else(|e| {
        eprintln!("error reading {path}: {e}");
        std::process::exit(1);
    });

    let Program { instructions, symbols } = assemble(&src);

    let mut symbols_by_addr: HashMap<Word16, Vec<String>> = HashMap::new();
    for (name, addr) in &symbols {
        let addr: Word16 = (*addr).into();
        symbols_by_addr.entry(addr).or_default().push(name.clone());
    }

    println!("Loaded {} instructions from {path}", instructions.len());

    let computer = Computer::chip();
    if print {
        println!("{}", print_graph(&computer));

        let squashed = half_flatten(Computer::chip());
        println!("{}", print_ic_graph(&squashed));
    }
    let chip = flatten(computer);
    let wiring = synthesize(&chip, memory_system());
    if print {
        print!("{wiring}");
    }
    let mut state = initialize(wiring);

    find_rom(&state).flash(instructions.iter().map(|&v| Word16::from(v)).collect());

    if no_exec {
        return;
    }

    let ram = find_ram(&state);
    let screen = find_screen(&state);
    let keyboard = find_keyboard(&state);
    let win_width  = (WIDTH  + 2 * BEZEL) * scale;
    let win_height = (HEIGHT + 2 * BEZEL) * scale;
    let mut pixels = load_bezel(scale);

    let mut window = Window::new(path, win_width, win_height, WindowOptions::default())
        .expect("failed to create window");

    let mut cycle: u64 = 0;
    let mut interval_start = Instant::now();
    let mut interval_cycles: u64 = 0;

    let print_state = |pc: Word16, cycle: u64| {
        let labels = symbols_by_addr.get(&pc).map(|v| format!(" [{}]", v.join(", "))).unwrap_or_default();
        println!("pc={pc}{labels}: (cycle {})", fmt_commas(cycle));
        println!("  SP: {}", ram.peek(0));
        let asm = instructions.get(pc.unsigned() as usize).map(|&i| disassemble(i)).unwrap_or("?".to_string());
        println!("  {asm}")
    };

    const TRACE_SKIP: &[&str] = &["math.abs", "math.multiply"];
    let print_fn_entry = |pc: Word16, cycle: u64| {
        if let Some(labels) = symbols_by_addr.get(&pc) {
            let fn_labels: Vec<&str> = labels.iter()
                .filter(|l| l.contains('.') && !l.contains('$') && !l.contains('_'))
                .filter(|l| !TRACE_SKIP.contains(&l.as_str()))
                .map(|l| l.as_str())
                .collect();
            if !fn_labels.is_empty() {
                println!("pc={pc} [{}] (cycle {})", fn_labels.join(", "), fmt_commas(cycle));
            }
        }
    };

    if verbose {
        print_state(state.get("pc"), cycle);
    } else if trace {
        print_fn_entry(state.get("pc"), cycle);
    }

    let mut halted = false;
    let halt_addr: Option<Word16> = symbols.get("sys.halt").copied().map(Into::into);
    let main_main_addr: Option<Word16> = symbols.get("main.main").copied().map(Into::into);
    let mut main_main_hit = false;
    let frame_addr: Option<Word16> = symbols.get("ponggame.moveball").copied().map(Into::into);
    let mut frame_count: u64 = 0;
    let mut last_frame_cycle: u64 = 0;
    let mut interval_frames: u64 = 0;

    while window.is_open() {
        if !halted {
            let frame_start = Instant::now();
            let mut batch: u64 = 0;
            loop {
                state.ticktock();
                cycle += 1;
                interval_cycles += 1;
                batch += 1;

                let pc = state.get("pc");
                if verbose {
                    let labels = symbols_by_addr.get(&pc).map(|v| format!(" [{}]", v.join(", "))).unwrap_or_default();
                    if !labels.is_empty() {
                        print_state(pc, cycle);
                    }
                } else if trace {
                    print_fn_entry(state.get("pc"), cycle);
                }

                if !main_main_hit && main_main_addr == Some(pc) {
                    println!("main.main reached at cycle {}", fmt_commas(cycle));
                    main_main_hit = true;
                }
                if frame_addr == Some(pc) {
                    frame_count += 1;
                    if trace {
                        let delta = cycle - last_frame_cycle;
                        println!("frame {} at cycle {} (+{})", fmt_commas(frame_count), fmt_commas(cycle), fmt_commas(delta));
                    }
                    last_frame_cycle = cycle;
                    interval_frames += 1;
                }

                if halt_addr == Some(pc) {
                    println!("Halted at sys.halt (cycle {})", fmt_commas(cycle));
                    halted = true;
                    break;
                }

                // Check the clock every 256 cycles to avoid calling it every iteration.
                if batch & 255 == 0 && frame_start.elapsed() >= FRAME_TIME {
                    break;
                }
            }

            keyboard.push((hack_keycode(&window) as u16).into());

            render_screen(&screen, &mut pixels, scale);
            window.update_with_buffer(&pixels, win_width, win_height).unwrap();

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
                let fps = interval_frames as f64 / elapsed.as_secs_f64();
                println!("cycles/s: {val:.1}{suffix} (total: {tval:.1}{tsuffix}, {fps:.1} fps)");
                interval_start = Instant::now();
                interval_cycles = 0;
                interval_frames = 0;
            }
        } else {
            // Halted: just keep the window open and responsive.
            window.update_with_buffer(&pixels, win_width, win_height).unwrap();
            std::thread::sleep(FRAME_TIME);
        }
    }
}
