use std::collections::HashMap;
use std::env;
use std::fs;
use std::time::Instant;

use minifb::{Window, WindowOptions};

use assignments::project_05::{Computer, flatten, find_ram, find_rom, find_screen, find_keyboard, memory_system};
use assignments::project_06::{assemble, Program};
use simulator::{print_graph, print_ic_graph};
use simulator::declare::Chip as _;
use simulator::simulate::{synthesize, initialize};
use simulator::word::Word16;

use computer::disasm::disassemble;
use computer::display::{self, WIDTH, HEIGHT, BEZEL, FRAME_TIME};
use computer::keyboard::hack_keycode;
use computer::{half_flatten, fmt_commas};

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
    let bezel = display::load_bezel(scale);
    let mut pixels = bezel.clone();

    let mut window = Window::new(path, win_width, win_height, WindowOptions::default())
        .expect("failed to create window");

    let mut cycle: u64 = 0;
    let mut interval_start = Instant::now();
    let mut interval_cycles: u64 = 0;
    let mut display_speed = String::new();
    let mut display_fps = String::new();

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

            display::render_screen(&screen, &mut pixels, scale);

            let bezel_top = (BEZEL + HEIGHT) * scale;
            for row in bezel_top..win_height {
                let start = row * win_width;
                pixels[start..start + win_width].copy_from_slice(&bezel[start..start + win_width]);
            }
            let text_y = bezel_top + (BEZEL - 9) * scale / 2;
            let text_color = 0x404040;
            if !display_speed.is_empty() {
                display::draw_text(&mut pixels, win_width, BEZEL * scale, text_y, scale, &display_speed, text_color);
                let fw = display::text_width(&display_fps, scale);
                display::draw_text(&mut pixels, win_width, win_width - BEZEL * scale - fw, text_y, scale, &display_fps, text_color);
            }

            window.update_with_buffer(&pixels, win_width, win_height).unwrap();

            let elapsed = interval_start.elapsed();
            if elapsed.as_millis() >= 200 {
                let cps = interval_cycles as f64 / elapsed.as_secs_f64();
                let fps = interval_frames as f64 / elapsed.as_secs_f64();
                display_speed = display::format_speed(cps);
                display_fps = format!("{:.0} fps", fps);
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
