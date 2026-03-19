/// Disassemble a single Hack machine instruction into human-readable assembly.
///
/// TODO: use symbolic names and labels from the source, or maybe even just look up the
/// corresponding source line(s)?
pub fn disassemble(instr: u16) -> String {
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
