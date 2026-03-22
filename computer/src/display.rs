use std::time::Duration;

use simulator::simulate::RAMHandle;
use simulator::nat::N16;

pub const WIDTH: usize = 512;
pub const HEIGHT: usize = 256;
pub const BEZEL: usize = 20;
pub const FRAME_TIME: Duration = Duration::from_millis(1000/120);
const BEZEL_PNG: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/bezel.png");


const WHITE_PIXEL: u32 = 0xFFFFFF;
const BLACK_PIXEL: u32 = 0x000000;

pub fn render_screen(screen: &RAMHandle<N16, N16>, pixels: &mut [u32], scale: usize) {
    let win_width = (WIDTH + 2 * BEZEL) * scale;

    // Fill the screen region with white.
    for row in 0..HEIGHT {
        for dy in 0..scale {
            let y = (BEZEL + row) * scale + dy;
            let start = y * win_width + BEZEL * scale;
            let end = start + WIDTH * scale;
            pixels[start..end].fill(WHITE_PIXEL);
        }
    }

    // Set only the black pixels.
    for word_idx in 0..(WIDTH / 16 * HEIGHT) {
        let word = screen.peek(word_idx as u64).unsigned() as u16;
        if word == 0 { continue; }
        let row = word_idx / (WIDTH / 16);
        let col_word = word_idx % (WIDTH / 16);
        let mut bits = word;
        while bits != 0 {
            let bit = bits.trailing_zeros() as usize;
            bits &= bits - 1;
            let px_x = (BEZEL + col_word * 16 + bit) * scale;
            let px_y = (BEZEL + row) * scale;
            for dy in 0..scale {
                let base = (px_y + dy) * win_width + px_x;
                pixels[base..base + scale].fill(BLACK_PIXEL);
            }
        }
    }
}

pub fn load_bezel(scale: usize) -> Vec<u32> {
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

// --- Bitmap font (Monaco 9) ---

/// Monaco 9 bitmap font, 5 pixels wide, 9 pixels tall (7 body + 2 descender).
/// See https://github.com/mossprescott/pynand/blob/master/alt/big/Monaco9.png
const FONT: [([u8; 9], char); 17] = [
    ([0x0E, 0x11, 0x11, 0x11, 0x11, 0x11, 0x0E, 0x00, 0x00], '0'),
    ([0x02, 0x06, 0x02, 0x02, 0x02, 0x02, 0x02, 0x00, 0x00], '1'),
    ([0x0E, 0x11, 0x01, 0x02, 0x04, 0x08, 0x1F, 0x00, 0x00], '2'),
    ([0x0E, 0x11, 0x01, 0x06, 0x01, 0x11, 0x0E, 0x00, 0x00], '3'),
    ([0x02, 0x06, 0x0A, 0x12, 0x1F, 0x02, 0x02, 0x00, 0x00], '4'),
    ([0x1F, 0x10, 0x1E, 0x01, 0x01, 0x11, 0x0E, 0x00, 0x00], '5'),
    ([0x0E, 0x10, 0x10, 0x1E, 0x11, 0x11, 0x0E, 0x00, 0x00], '6'),
    ([0x1F, 0x01, 0x01, 0x02, 0x04, 0x04, 0x04, 0x00, 0x00], '7'),
    ([0x0E, 0x11, 0x11, 0x0E, 0x11, 0x11, 0x0E, 0x00, 0x00], '8'),
    ([0x0E, 0x11, 0x11, 0x0F, 0x01, 0x01, 0x0E, 0x00, 0x00], '9'),
    ([0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x04, 0x00, 0x00], '.'),
    ([0x11, 0x1B, 0x15, 0x11, 0x11, 0x11, 0x11, 0x00, 0x00], 'M'),
    ([0x11, 0x11, 0x11, 0x1F, 0x11, 0x11, 0x11, 0x00, 0x00], 'H'),
    ([0x00, 0x00, 0x1F, 0x02, 0x04, 0x08, 0x1F, 0x00, 0x00], 'z'),
    ([0x03, 0x04, 0x0E, 0x04, 0x04, 0x04, 0x04, 0x00, 0x00], 'f'),
    ([0x00, 0x00, 0x1E, 0x11, 0x11, 0x11, 0x1E, 0x10, 0x10], 'p'),
    ([0x00, 0x00, 0x0F, 0x10, 0x0E, 0x01, 0x1E, 0x00, 0x00], 's'),
];

fn glyph(ch: char) -> [u8; 9] {
    for &(bits, c) in &FONT {
        if c == ch { return bits; }
    }
    [0; 9]
}

pub fn draw_text(pixels: &mut [u32], win_width: usize, x: usize, y: usize, scale: usize, text: &str, color: u32) {
    let mut cx = x;
    for ch in text.chars() {
        let g = glyph(ch);
        for row in 0..9 {
            for col in 0..5 {
                if g[row] & (0x10 >> col) != 0 {
                    for dy in 0..scale {
                        for dx in 0..scale {
                            let px = cx + col * scale + dx;
                            let py = y + row * scale + dy;
                            if px < win_width {
                                pixels[py * win_width + px] = color;
                            }
                        }
                    }
                }
            }
        }
        cx += 6 * scale;
    }
}

pub fn text_width(text: &str, scale: usize) -> usize {
    let n = text.len();
    if n == 0 { 0 } else { (n * 6 - 1) * scale }
}

pub fn format_speed(cps: f64) -> String {
    format!("{:.2} MHz", cps / 1_000_000.0)
}
