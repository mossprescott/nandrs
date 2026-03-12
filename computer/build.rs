// Generates bezel.png (552×296) in the crate root if it doesn't already exist.
// Delete it and rebuild to regenerate; edit it freely without rebuilding.
fn main() {
    println!("cargo:rerun-if-changed=build.rs");

    let dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let path = format!("{dir}/bezel.png");
    if std::path::Path::new(&path).exists() {
        return;
    }

    // Screen is 512×256; bezel is 20px on each side → 552×296.
    const W: u32 = 552;
    const H: u32 = 296;
    // 80s beige: warm, slightly creamy — like an old IBM/Commodore chassis.
    const BEIGE: [u8; 3] = [0xC8, 0xB8, 0x98];

    let row: Vec<u8> = BEIGE.iter().cycle().take((W * 3) as usize).cloned().collect();
    let data: Vec<u8> = row.iter().cycle().take((W * H * 3) as usize).cloned().collect();

    let file = std::fs::File::create(&path).expect("failed to create bezel.png");
    let mut enc = png::Encoder::new(file, W, H);
    enc.set_color(png::ColorType::Rgb);
    enc.set_depth(png::BitDepth::Eight);
    let mut writer = enc.write_header().expect("png header");
    writer.write_image_data(&data).expect("png data");

    println!("cargo:warning=created {path}");
}
