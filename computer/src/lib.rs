pub mod cli;
pub mod disasm;
pub mod display;
pub mod keyboard;

use std::collections::HashMap;
use std::time::Instant;

use minifb::{Window, WindowOptions};

use assignments::project_05::{find_keyboard, find_ram, find_screen};
use simulator::simulate::ChipState;
use simulator::word::Word16;

pub fn fmt_commas(n: u64) -> String {
    let s = n.to_string();
    let mut out = String::new();
    for (i, c) in s.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            out.push(',');
        }
        out.push(c);
    }
    out.chars().rev().collect()
}

pub fn run(
    args: &cli::Args,
    mut state: ChipState<simulator::nat::N16, simulator::nat::N16>,
    symbols: &HashMap<String, u16>,
    fmt_instr: &dyn Fn(Word16) -> String,
) {
    let mut symbols_by_addr: HashMap<Word16, Vec<String>> = HashMap::new();
    for (name, addr) in symbols {
        let addr: Word16 = (*addr).into();
        symbols_by_addr.entry(addr).or_default().push(name.clone());
    }

    let ram = find_ram(&state);
    let screen = find_screen(&state);
    let keyboard = find_keyboard(&state);
    let win_width = (display::WIDTH + 2 * display::BEZEL) * args.scale();
    let win_height = (display::HEIGHT + 2 * display::BEZEL) * args.scale();
    let bezel = display::load_bezel(args.scale());
    let mut pixels = bezel.clone();

    let mut window = Window::new(&args.path, win_width, win_height, WindowOptions::default())
        .expect("failed to create window");

    let mut cycle: u64 = 0;
    let mut interval_start = Instant::now();
    let mut interval_cycles: u64 = 0;
    let mut display_speed = String::new();
    let mut display_fps = String::new();

    let print_state = |pc: Word16, cycle: u64| {
        let labels = symbols_by_addr
            .get(&pc)
            .map(|v| format!(" [{}]", v.join(", ")))
            .unwrap_or_default();
        println!("pc={pc}{labels}: (cycle {})", fmt_commas(cycle));
        println!("  SP: {}", ram.peek(0));
        let asm = fmt_instr(pc);
        println!("  {asm}")
    };

    const TRACE_SKIP: &[&str] = &["math.abs", "math.multiply"];
    let print_fn_entry = |pc: Word16, cycle: u64| {
        if let Some(labels) = symbols_by_addr.get(&pc) {
            let fn_labels: Vec<&str> = labels
                .iter()
                .filter(|l| l.contains('.') && !l.contains('$') && !l.contains('_'))
                .filter(|l| !TRACE_SKIP.contains(&l.as_str()))
                .map(|l| l.as_str())
                .collect();
            if !fn_labels.is_empty() {
                println!(
                    "pc={pc} [{}] (cycle {})",
                    fn_labels.join(", "),
                    fmt_commas(cycle)
                );
            }
        }
    };

    state.reset();
    cycle += 1;

    if args.verbose {
        print_state(state.get("pc"), cycle);
    } else if args.trace {
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
                if args.verbose {
                    // let labels = symbols_by_addr.get(&pc).map(|v| format!(" [{}]", v.join(", "))).unwrap_or_default();
                    // if !labels.is_empty() {
                    print_state(pc, cycle);
                    // }
                } else if args.trace {
                    print_fn_entry(state.get("pc"), cycle);
                }

                if !main_main_hit && main_main_addr == Some(pc) {
                    println!("main.main reached at cycle {}", fmt_commas(cycle));
                    main_main_hit = true;
                }
                if frame_addr == Some(pc) {
                    frame_count += 1;
                    if args.trace {
                        let delta = cycle - last_frame_cycle;
                        println!(
                            "frame {} at cycle {} (+{})",
                            fmt_commas(frame_count),
                            fmt_commas(cycle),
                            fmt_commas(delta)
                        );
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
                if batch & 255 == 0 && frame_start.elapsed() >= display::FRAME_TIME {
                    break;
                }
            }

            keyboard.push((keyboard::hack_keycode(&window) as u16).into());

            display::render_screen(&screen, &mut pixels, args.scale());

            let bezel_top = (display::BEZEL + display::HEIGHT) * args.scale();
            for row in bezel_top..win_height {
                let start = row * win_width;
                pixels[start..start + win_width].copy_from_slice(&bezel[start..start + win_width]);
            }
            let text_y = bezel_top + (display::BEZEL - 9) * args.scale() / 2;
            let text_color = 0x404040;
            if !display_speed.is_empty() {
                display::draw_text(
                    &mut pixels,
                    win_width,
                    display::BEZEL * args.scale(),
                    text_y,
                    args.scale(),
                    &display_speed,
                    text_color,
                );
                let fw = display::text_width(&display_fps, args.scale());
                display::draw_text(
                    &mut pixels,
                    win_width,
                    win_width - display::BEZEL * args.scale() - fw,
                    text_y,
                    args.scale(),
                    &display_fps,
                    text_color,
                );
            }

            window
                .update_with_buffer(&pixels, win_width, win_height)
                .unwrap();

            let elapsed = interval_start.elapsed();
            if elapsed.as_millis() >= 1000 {
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
                println!("cycles/s: {val:.1}{suffix} (total: {tval:.1}{tsuffix}); {fps:.1} fps");
                interval_start = Instant::now();
                interval_cycles = 0;
                interval_frames = 0;
            }
        } else {
            // Halted: just keep the window open and responsive.
            window
                .update_with_buffer(&pixels, win_width, win_height)
                .unwrap();
            std::thread::sleep(display::FRAME_TIME);
        }
    }
}
