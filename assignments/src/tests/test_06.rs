use crate::project_06::parse_statement;

#[test]
fn parse_labels() {
    use crate::project_06::Statement::Label;

    assert_eq!(parse_statement("(main)"), Some(Label("main".into())));
    assert_eq!(
        parse_statement("(_branch_123_gen-foo$hello)"),
        Some(Label("_branch_123_gen-foo$hello".into()))
    );
}

#[test]
fn parse_addresses() {
    use crate::project_06::Statement::Address;

    assert_eq!(parse_statement("@SP"), Some(Address("SP".into())));
    assert_eq!(parse_statement("@main"), Some(Address("main".into())));
    assert_eq!(
        parse_statement("@_branch_123_gen-foo$hello"),
        Some(Address("_branch_123_gen-foo$hello".into()))
    );
}

#[test]
fn parse_constants() {
    use crate::project_06::Statement::Literal;

    assert_eq!(parse_statement("@0"), Some(Literal(0x0000)));
    assert_eq!(parse_statement("@256"), Some(Literal(0x0100)));
    assert_eq!(parse_statement("@32767"), Some(Literal(0x7fff)));
}

/// To avoid any ambiguity, test every bit pattern. There are surprisingly few; what does that
/// tell you about this ISA?
#[test]
fn parse_basic_ops() {
    use crate::project_06::Statement::Instruction;

    // base: 111_a_cccccc_000_000; no dest, no jump
    assert_eq!(parse_statement("0"), Some(Instruction(0xEA80))); // a=0 c=101010
    assert_eq!(parse_statement("1"), Some(Instruction(0xEFC0))); // a=0 c=111111
    assert_eq!(parse_statement("-1"), Some(Instruction(0xEE80))); // a=0 c=111010
    assert_eq!(parse_statement("D"), Some(Instruction(0xE300))); // a=0 c=001100
    assert_eq!(parse_statement("A"), Some(Instruction(0xEC00))); // a=0 c=110000
    assert_eq!(parse_statement("!D"), Some(Instruction(0xE340))); // a=0 c=001101
    assert_eq!(parse_statement("!A"), Some(Instruction(0xEC40))); // a=0 c=110001
    assert_eq!(parse_statement("-D"), Some(Instruction(0xE3C0))); // a=0 c=001111
    assert_eq!(parse_statement("-A"), Some(Instruction(0xECC0))); // a=0 c=110011
    assert_eq!(parse_statement("D+1"), Some(Instruction(0xE7C0))); // a=0 c=011111
    assert_eq!(parse_statement("A+1"), Some(Instruction(0xEDC0))); // a=0 c=110111
    assert_eq!(parse_statement("D-1"), Some(Instruction(0xE380))); // a=0 c=001110
    assert_eq!(parse_statement("A-1"), Some(Instruction(0xEC80))); // a=0 c=110010
    assert_eq!(parse_statement("D+A"), Some(Instruction(0xE080))); // a=0 c=000010
    assert_eq!(parse_statement("D-A"), Some(Instruction(0xE4C0))); // a=0 c=010011
    assert_eq!(parse_statement("A-D"), Some(Instruction(0xE1C0))); // a=0 c=000111
    assert_eq!(parse_statement("D&A"), Some(Instruction(0xE000))); // a=0 c=000000
    assert_eq!(parse_statement("D|A"), Some(Instruction(0xE540))); // a=0 c=010101
    // M variants: same comp bits, a=1 (+0x1000)
    assert_eq!(parse_statement("M"), Some(Instruction(0xFC00))); // a=1 c=110000
    assert_eq!(parse_statement("!M"), Some(Instruction(0xFC40))); // a=1 c=110001
    assert_eq!(parse_statement("-M"), Some(Instruction(0xFCC0))); // a=1 c=110011
    assert_eq!(parse_statement("M+1"), Some(Instruction(0xFDC0))); // a=1 c=110111
    assert_eq!(parse_statement("M-1"), Some(Instruction(0xFC80))); // a=1 c=110010
    assert_eq!(parse_statement("D+M"), Some(Instruction(0xF080))); // a=1 c=000010
    assert_eq!(parse_statement("D-M"), Some(Instruction(0xF4C0))); // a=1 c=010011
    assert_eq!(parse_statement("M-D"), Some(Instruction(0xF1C0))); // a=1 c=000111
    assert_eq!(parse_statement("D&M"), Some(Instruction(0xF000))); // a=1 c=000000
    assert_eq!(parse_statement("D|M"), Some(Instruction(0xF540))); // a=1 c=010101
}

#[test]
fn parse_destinations() {
    use crate::project_06::Statement::Instruction;

    assert_eq!(
        parse_statement("A=0"),
        Some(Instruction(0xEA80 | 0b100_000))
    );
    assert_eq!(
        parse_statement("D=0"),
        Some(Instruction(0xEA80 | 0b010_000))
    );
    assert_eq!(
        parse_statement("M=0"),
        Some(Instruction(0xEA80 | 0b001_000))
    );
}

#[test]
fn parse_jump_conditions() {
    use crate::project_06::Statement::Instruction;

    assert_eq!(parse_statement("D;JLT"), Some(Instruction(0xE300 | 0b100)));
    assert_eq!(parse_statement("D;JEQ"), Some(Instruction(0xE300 | 0b010)));
    assert_eq!(parse_statement("D;JGT"), Some(Instruction(0xE300 | 0b001)));

    assert_eq!(parse_statement("D;JLE"), Some(Instruction(0xE300 | 0b110)));
    assert_eq!(parse_statement("D;JNE"), Some(Instruction(0xE300 | 0b101)));
    assert_eq!(parse_statement("D;JGE"), Some(Instruction(0xE300 | 0b011)));

    assert_eq!(parse_statement("JMP"), Some(Instruction(0xEA80 | 0b111)));

    // Don't jump. Kinda redundant.
    assert_eq!(parse_statement("0"), Some(Instruction(0xEA80 | 0b000)));
}

// - Some error cases to keep me out of trouble:

#[test]
fn reject_bad_labels() {
    assert_eq!(parse_statement("()"), None);
    assert_eq!(parse_statement("(0)"), None);
    assert_eq!(parse_statement("(0anything)"), None);
    assert_eq!(parse_statement("(foo bar)"), None);
}

#[test]
fn reject_bad_addresses() {
    assert_eq!(parse_statement("@"), None);
    assert_eq!(parse_statement("@0anything"), None);

    // If this doesn't get split earlier, it's an error here:
    assert_eq!(parse_statement("@foo bar"), None);
}

#[test]
fn reject_bad_constants() {
    assert_eq!(parse_statement("@-1"), None);
    assert_eq!(parse_statement("@1.5"), None);
    assert_eq!(parse_statement("@32768"), None);
}

// - Now, some compatible extensions that can be useful:

#[test]
fn parse_multiple_destinations() {
    use crate::project_06::Statement::Instruction;

    // Any order:
    assert_eq!(
        parse_statement("DA=0"),
        Some(Instruction(0xEA80 | 0b110_000))
    );
    assert_eq!(
        parse_statement("DM=0"),
        Some(Instruction(0xEA80 | 0b011_000))
    );
    assert_eq!(
        parse_statement("AMD=0"),
        Some(Instruction(0xEA80 | 0b111_000))
    );

    // Commonly-used to adjust the stack:
    assert_eq!(
        parse_statement("AM=M-1"),
        Some(Instruction(0xFC80 | 0b101_000))
    );
}

#[test]
fn parse_hex_constants() {
    use crate::project_06::Statement::Literal;

    assert_eq!(parse_statement("@0x0"), Some(Literal(0x0000)));
    assert_eq!(parse_statement("@0x100"), Some(Literal(0x0100)));
    assert_eq!(parse_statement("@0x7fff"), Some(Literal(0x7fff)));

    // Some variations:
    assert_eq!(parse_statement("@0x00"), Some(Literal(0x0000)));
    assert_eq!(parse_statement("@0x0100"), Some(Literal(0x0100)));
    assert_eq!(parse_statement("@0x7ABC"), Some(Literal(0x7abc)));

    assert_eq!(parse_statement("@0x8000"), None);
}
