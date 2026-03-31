use crate::project_05::{KEYBOARD, SCREEN_BASE};
/// HACK Assembly translation
use std::collections::HashMap;

#[derive(Debug, PartialEq)]
pub struct Label(String);
impl From<&str> for Label {
    fn from(value: &str) -> Self {
        Label(value.to_string())
    }
}

pub type Addr = u16;

/// Unit of assembly source, either an instruction, like "D=A" or "@256", or a label, like
/// "(main)"
#[derive(Debug, PartialEq)]
pub enum Statement {
    /// Names a location in the ROM when the program is loaded. Does not consume space in ROM.
    /// Such as "(start)".
    Label(Label),

    /// A so-called "A-instruction", which loads the address of a label referred to by name.
    /// For example: "@start". Note: these values are limited to 15-bits (addresses within 32K-words).
    Address(Label),

    /// A so-called "A-instruction", which loads an explicit constant value.
    /// For example: "@256". Note: these values are limited to 15-bits (non-negative integers).
    Literal(u16),

    /// A so-called "C-instruction", which directs the ALU to do some calculation.
    /// Such as "D=A".
    Instruction(u16),
}

impl Statement {
    /// Definite bit pattern represented by the statement, if any.
    ///
    /// As a sanity check, results are checked for being in range (no negative literals or
    /// instructions that impersonate literals.)
    pub fn raw(&self) -> Option<u16> {
        match self {
            Statement::Literal(x) => {
                if *x <= 0x7fff {
                    Some(*x)
                } else {
                    None
                }
            }
            Statement::Instruction(x) => {
                if *x > 0x7fff {
                    Some(*x)
                } else {
                    None
                }
            }
            _ => None,
        }
    }
}

/// Decode a single assembly `Statement` from one line of source text.
///
/// Returns `None` for blank lines, comment-only lines, and unrecognized input.
///
/// Handles more variations than are explicitly required for the normal tools:
/// - missing destination: "D+M"; calculates a value and does nothing with it
/// - multiple destinations: "DM=0", "DA=!D"; set both destinations to the output value
///   (even A and M but that might be undefined depending on your CPU)
/// - hex constants: "@0x007f"
fn is_valid_symbol(s: &str) -> bool {
    !s.is_empty()
        && s.chars()
            .next()
            .map_or(false, |c| c.is_alphabetic() || c == '_')
        && !s.chars().any(|c| c.is_whitespace())
}

pub fn parse_statement(line: &str) -> Option<Statement> {
    // Strip comments and whitespace
    let line = line.split("//").next().unwrap().trim();
    if line.is_empty() {
        return None;
    }

    // Label: (name)
    if line.starts_with('(') && line.ends_with(')') {
        let name = &line[1..line.len() - 1];
        return if is_valid_symbol(name) {
            Some(Statement::Label(Label(name.to_string())))
        } else {
            None
        };
    }

    // A-instruction: @value or @symbol
    if let Some(rest) = line.strip_prefix('@') {
        if rest.is_empty() {
            return None;
        }
        // Hex literal: @0x...
        if let Some(hex) = rest.strip_prefix("0x").or_else(|| rest.strip_prefix("0X")) {
            return u16::from_str_radix(hex, 16)
                .ok()
                .filter(|&n| n <= 0x7fff)
                .map(Statement::Literal);
        }
        // Decimal literal: starts with a digit, must be in 15-bit range
        if rest.chars().next().unwrap().is_ascii_digit() {
            return rest
                .parse::<u16>()
                .ok()
                .filter(|&n| n <= 0x7fff)
                .map(Statement::Literal);
        }
        // Symbolic address
        return if is_valid_symbol(rest) {
            Some(Statement::Address(Label(rest.to_string())))
        } else {
            None
        };
    }

    // C-instruction: [dest=]comp[;jump]
    let (dest, rest) = if let Some(eq) = line.find('=') {
        (&line[..eq], &line[eq + 1..])
    } else {
        ("", line)
    };
    let (comp, jump) = if let Some(semi) = rest.find(';') {
        (&rest[..semi], &rest[semi + 1..])
    } else if matches!(rest, "JGT" | "JEQ" | "JGE" | "JLT" | "JNE" | "JLE" | "JMP") {
        ("0", rest)
    } else {
        (rest, "")
    };

    // dest bits: d1=write_a (bit 5), d2=write_d (bit 4), d3=write_m (bit 3)
    let dest_bits: u16 = dest.chars().fold(0, |acc, c| {
        acc | match c {
            'A' => 0b100,
            'D' => 0b010,
            'M' => 0b001,
            _ => 0,
        }
    });

    // jump bits
    let jump_bits: u16 = match jump {
        "" => 0b000,
        "JGT" => 0b001,
        "JEQ" => 0b010,
        "JGE" => 0b011,
        "JLT" => 0b100,
        "JNE" => 0b101,
        "JLE" => 0b110,
        "JMP" => 0b111,
        _ => return None,
    };

    // comp: (a-bit, cccccc)
    let (a_bit, comp_bits): (u16, u16) = match comp {
        "0" => (0, 0b101010),
        "1" => (0, 0b111111),
        "-1" => (0, 0b111010),
        "D" => (0, 0b001100),
        "A" => (0, 0b110000),
        "!D" => (0, 0b001101),
        "!A" => (0, 0b110001),
        "-D" => (0, 0b001111),
        "-A" => (0, 0b110011),
        "D+1" => (0, 0b011111),
        "A+1" => (0, 0b110111),
        "D-1" => (0, 0b001110),
        "A-1" => (0, 0b110010),
        "D+A" | "A+D" => (0, 0b000010),
        "D-A" => (0, 0b010011),
        "A-D" => (0, 0b000111),
        "D&A" | "A&D" => (0, 0b000000),
        "D|A" | "A|D" => (0, 0b010101),
        "M" => (1, 0b110000),
        "!M" => (1, 0b110001),
        "-M" => (1, 0b110011),
        "M+1" => (1, 0b110111),
        "M-1" => (1, 0b110010),
        "D+M" | "M+D" => (1, 0b000010),
        "D-M" => (1, 0b010011),
        "M-D" => (1, 0b000111),
        "D&M" | "M&D" => (1, 0b000000),
        "D|M" | "M|D" => (1, 0b010101),
        _ => return None,
    };

    let bits = 0b111_0_000000_000_000_u16
        | (a_bit << 12)
        | (comp_bits << 6)
        | (dest_bits << 3)
        | jump_bits;

    Some(Statement::Instruction(bits))
}

pub struct Program {
    pub instructions: Vec<u16>,
    pub symbols: HashMap<String, Addr>,
}

/// Consume source text, producing a buffer of 16-bit instruction values.
pub fn assemble(src: &str) -> Program {
    // Predefined symbols
    let mut symbols: HashMap<String, u16> = HashMap::new();
    for i in 0u16..=15 {
        symbols.insert(format!("R{i}"), i);
    }
    symbols.insert("SP".into(), 0);
    symbols.insert("LCL".into(), 1);
    symbols.insert("ARG".into(), 2);
    symbols.insert("THIS".into(), 3);
    symbols.insert("THAT".into(), 4);
    symbols.insert("SCREEN".into(), SCREEN_BASE);
    symbols.insert("KBD".into(), KEYBOARD);

    let stmts: Vec<Statement> = src.lines().filter_map(parse_statement).collect();

    // Pass 1: assign ROM addresses to labels.
    let mut labels: HashMap<String, u16> = HashMap::new();
    let mut rom_addr: u16 = 0;
    for stmt in &stmts {
        match stmt {
            Statement::Label(Label(name)) => {
                symbols.insert(name.clone(), rom_addr);
                labels.insert(name.clone(), rom_addr);
            }
            _ => {
                rom_addr += 1;
            }
        }
    }

    // Pass 2: emit instructions, allocating variables starting at RAM[16].
    let mut next_var: u16 = 16;
    let mut out = Vec::new();
    for stmt in stmts {
        match stmt {
            Statement::Label(_) => {}
            Statement::Literal(x) => out.push(x),
            Statement::Instruction(x) => out.push(x),
            Statement::Address(Label(name)) => {
                let addr = *symbols.entry(name).or_insert_with(|| {
                    let v = next_var;
                    next_var += 1;
                    v
                });
                out.push(addr);
            }
        }
    }
    Program {
        instructions: out,
        symbols: labels,
    }
}
