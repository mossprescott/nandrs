use simulator::{Component, print_graph, Chip as _};
use simulator::component::Combinational;
use simulator::eval::eval;
use crate::project_01;
use crate::project_02::{flatten, MyHalfAdder, MyFullAdder, Inc16, Add16, Zero16, Neg16, ALU};

/// Flatten a chip that expands directly to Project01Components (e.g. MyHalfAdder, MyFullAdder).
fn flatten_nands<C: simulator::Reflect + Component<Target = project_01::Project01Component>>(chip: C) -> simulator::IC<Combinational<simulator::nat::N16>> {
    let ic = chip.expand().expect("expected expandable component");
    simulator::IC {
        name: format!("{} (flat)", chip.name()),
        intf: chip.reflect(),
        components: ic.components.into_iter().flat_map(|c| project_01::flatten(c).components).collect(),
    }
}

#[test]
fn half_adder_truth_table() {
    let chip = flatten_nands(MyHalfAdder::chip());
    let r = eval(&chip, [("a", false.into()), ("b", false.into())]); assert_eq!(r["sum"].unsigned(), 0); assert_eq!(r["carry"].unsigned(), 0);
    let r = eval(&chip, [("a", false.into()), ("b", true.into())]); assert_eq!(r["sum"].unsigned(), 1); assert_eq!(r["carry"].unsigned(), 0);
    let r = eval(&chip, [("a", true.into()), ("b", false.into())]); assert_eq!(r["sum"].unsigned(), 1); assert_eq!(r["carry"].unsigned(), 0);
    let r = eval(&chip, [("a", true.into()), ("b", true.into())]); assert_eq!(r["sum"].unsigned(), 0); assert_eq!(r["carry"].unsigned(), 1);
}

#[test]
fn half_adder_optimal() {
    assert_eq!(flatten_nands(MyHalfAdder::chip()).components.len(), 5);
}

#[test]
fn full_adder_truth_table() {
    let chip = flatten_nands(MyFullAdder::chip());
    let r = eval(&chip, [("a", false.into()), ("b", false.into()), ("c", false.into())]); assert_eq!(r["sum"].unsigned(), 0); assert_eq!(r["carry"].unsigned(), 0);
    let r = eval(&chip, [("a", false.into()), ("b", false.into()), ("c", true.into())]); assert_eq!(r["sum"].unsigned(), 1); assert_eq!(r["carry"].unsigned(), 0);
    let r = eval(&chip, [("a", false.into()), ("b", true.into()), ("c", false.into())]); assert_eq!(r["sum"].unsigned(), 1); assert_eq!(r["carry"].unsigned(), 0);
    let r = eval(&chip, [("a", false.into()), ("b", true.into()), ("c", true.into())]); assert_eq!(r["sum"].unsigned(), 0); assert_eq!(r["carry"].unsigned(), 1);
    let r = eval(&chip, [("a", true.into()), ("b", false.into()), ("c", false.into())]); assert_eq!(r["sum"].unsigned(), 1); assert_eq!(r["carry"].unsigned(), 0);
    let r = eval(&chip, [("a", true.into()), ("b", false.into()), ("c", true.into())]); assert_eq!(r["sum"].unsigned(), 0); assert_eq!(r["carry"].unsigned(), 1);
    let r = eval(&chip, [("a", true.into()), ("b", true.into()), ("c", false.into())]); assert_eq!(r["sum"].unsigned(), 0); assert_eq!(r["carry"].unsigned(), 1);
    let r = eval(&chip, [("a", true.into()), ("b", true.into()), ("c", true.into())]); assert_eq!(r["sum"].unsigned(), 1); assert_eq!(r["carry"].unsigned(), 1);
}

#[test]
fn full_adder_optimal() {
    assert_eq!(flatten_nands(MyFullAdder::chip()).components.len(), 9);
}

#[test]
fn inc16_truth_table() {
    let chip = flatten(Inc16::chip());
    assert_eq!(eval(&chip, [("a", 0u16.into())])["out"].unsigned(),     1);
    assert_eq!(eval(&chip, [("a", 1u16.into())])["out"].unsigned(),     2);
    assert_eq!(eval(&chip, [("a", 42u16.into())])["out"].unsigned(),    43);
    assert_eq!(eval(&chip, [("a", 0xFFFFu16.into())])["out"].unsigned(), 0); // overflow wraps
}

#[test]
fn inc16_optimal() {
    let components = flatten(Inc16::chip()).components;
    let nands = components.iter().filter(|c| matches!(c, Combinational::Nand(_))).count();
    let adders = components.iter().filter(|c| matches!(c, Combinational::Adder(_))).count();
    // Not(1) for bit 0, plus 15 FullAdders for the carry chain
    assert_eq!(nands, 1);
    assert_eq!(adders, 15);
}

#[test]
fn add16_truth_table() {
    let chip = flatten(Add16::chip());
    assert_eq!(eval(&chip, [("a", 0u16.into()),    ("b", 0u16.into())])["out"].signed(),    0);
    assert_eq!(eval(&chip, [("a", 1u16.into()),    ("b", 1u16.into())])["out"].signed(),    2);
    assert_eq!(eval(&chip, [("a", 100u16.into()),  ("b", 200u16.into())])["out"].signed(),  300);
    assert_eq!(eval(&chip, [("a", 0xFFFFu16.into()), ("b", 1u16.into())])["out"].signed(),  0); // overflow wraps

    assert_eq!(eval(&chip, [("a", (-1i16).into()),    ("b", (-2i16).into())])["out"].signed(),    -3);
    assert_eq!(eval(&chip, [("a", (-32768i16).into()), ("b", (-1i16).into())])["out"].signed(),    32767);
}

#[test]
fn add16_optimal() {
    let components = flatten(Add16::chip()).components;
    let nands = components.iter().filter(|c| matches!(c, Combinational::Nand(_))).count();
    let adders = components.iter().filter(|c| matches!(c, Combinational::Adder(_))).count();
    assert_eq!(nands, 0);
    assert_eq!(adders, 16);
}

#[test]
fn zero16_truth_table() {
    let chip = flatten(Zero16::chip());
    assert_eq!(eval(&chip, [("a", 0u16.into())])["out"].unsigned(),      1); // all zeros
    assert_eq!(eval(&chip, [("a", 1u16.into())])["out"].unsigned(),      0); // bit 0 set
    assert_eq!(eval(&chip, [("a", 0x8000u16.into())])["out"].unsigned(), 0); // only MSB set
    assert_eq!(eval(&chip, [("a", 0xFFFFu16.into())])["out"].unsigned(), 0); // all ones
}

#[test]
fn zero16_optimal() {
    // Or-tree over 16 bits (15 Ors x 3 Nands) + Not(1) = 46
    assert_eq!(flatten(Zero16::chip()).components.len(), 46);
}

#[test]
fn neg16_truth_table() {
    let chip = flatten(Neg16::chip());
    assert_eq!(eval(&chip, [("a", 0u16.into())])["out"].unsigned(),      0); // zero is not negative
    assert_eq!(eval(&chip, [("a", 1u16.into())])["out"].unsigned(),      0); // positive
    assert_eq!(eval(&chip, [("a", 0x7FFFu16.into())])["out"].unsigned(), 0); // max positive
    assert_eq!(eval(&chip, [("a", 0x8000u16.into())])["out"].unsigned(), 1); // min negative (-32768)
    assert_eq!(eval(&chip, [("a", 0xFFFFu16.into())])["out"].unsigned(), 1); // -1
}

#[test]
fn neg16_optimal() {
    let components = flatten(Neg16::chip()).components;
    let nands = components.iter().filter(|c| matches!(c, Combinational::Nand(_))).count();
    assert_eq!(nands, 0);
}

#[test]
fn alu_truth_table() {
    let chip = ALU::chip();

    // When it breaks, it's nice to see what it tried to do
    print!("{}", print_graph(&chip));

    let chip = flatten(chip);

    // 0 = 0 + 0
    let r = eval(&chip, [("x", 0u16.into()), ("y", 0u16.into()), ("zx", true.into()), ("nx", false.into()), ("zy", true.into()), ("ny", false.into()), ("f", true.into()), ("no", false.into())]);
    assert_eq!(r["out"].unsigned(), 0);      assert_eq!(r["zr"].unsigned(), 1); assert_eq!(r["ng"].unsigned(), 0); // 0

    // 1 = !(-1 + -1)
    let r = eval(&chip, [("x", 0u16.into()), ("y", 0u16.into()), ("zx", true.into()), ("nx", true.into()), ("zy", true.into()), ("ny", true.into()), ("f", true.into()), ("no", true.into())]);
    assert_eq!(r["out"].unsigned(), 1);      assert_eq!(r["zr"].unsigned(), 0); assert_eq!(r["ng"].unsigned(), 0); // 1

    // -1 = -1 + 0
    let r = eval(&chip, [("x", 0u16.into()), ("y", 0u16.into()), ("zx", true.into()), ("nx", true.into()), ("zy", true.into()), ("ny", false.into()), ("f", true.into()), ("no", false.into())]);
    assert_eq!(r["out"].unsigned(), 0xffff); assert_eq!(r["zr"].unsigned(), 0); assert_eq!(r["ng"].unsigned(), 1); // -1

    // x = x and 0xfff
    let r = eval(&chip, [("x", 5u16.into()), ("y", 3u16.into()), ("zx", false.into()), ("nx", false.into()), ("zy", true.into()), ("ny", true.into()), ("f", false.into()), ("no", false.into())]);
    assert_eq!(r["out"].unsigned(), 5);      assert_eq!(r["zr"].unsigned(), 0); assert_eq!(r["ng"].unsigned(), 0); // x

    // y = 0xfff and y
    let r = eval(&chip, [("x", 5u16.into()), ("y", 3u16.into()), ("zx", true.into()), ("nx", true.into()), ("zy", false.into()), ("ny", false.into()), ("f", false.into()), ("no", false.into())]);
    assert_eq!(r["out"].unsigned(), 3);      assert_eq!(r["zr"].unsigned(), 0); assert_eq!(r["ng"].unsigned(), 0); // y

    // x + y
    let r = eval(&chip, [("x", 5u16.into()), ("y", 3u16.into()), ("zx", false.into()), ("nx", false.into()), ("zy", false.into()), ("ny", false.into()), ("f", true.into()), ("no", false.into())]);
    assert_eq!(r["out"].unsigned(), 8);      assert_eq!(r["zr"].unsigned(), 0); assert_eq!(r["ng"].unsigned(), 0); // x + y

    // x - y = !(!x + y)
    let r = eval(&chip, [("x", 5u16.into()), ("y", 3u16.into()), ("zx", false.into()), ("nx", true.into()), ("zy", false.into()), ("ny", false.into()), ("f", true.into()), ("no", true.into())]);
    assert_eq!(r["out"].unsigned(), 2);      assert_eq!(r["zr"].unsigned(), 0); assert_eq!(r["ng"].unsigned(), 0); // x - y

    // x and y
    let r = eval(&chip, [("x", 0b1010u16.into()), ("y", 0b1100u16.into()), ("zx", false.into()), ("nx", false.into()), ("zy", false.into()), ("ny", false.into()), ("f", false.into()), ("no", false.into())]);
    assert_eq!(r["out"].unsigned(), 0b1000); assert_eq!(r["zr"].unsigned(), 0); assert_eq!(r["ng"].unsigned(), 0); // x AND y

    // x or y = !(!x and !y)
    let r = eval(&chip, [("x", 0b1010u16.into()), ("y", 0b0101u16.into()), ("zx", false.into()), ("nx", true.into()), ("zy", false.into()), ("ny", true.into()), ("f", false.into()), ("no", true.into())]);
    assert_eq!(r["out"].unsigned(), 0b1111); assert_eq!(r["zr"].unsigned(), 0); assert_eq!(r["ng"].unsigned(), 0); // x OR y
}

#[test]
fn alu_optimal() {
    let components = flatten(ALU::chip()).components;
    let nands = components.iter().filter(|c| matches!(c, Combinational::Nand(_))).count();
    let adders = components.iter().filter(|c| matches!(c, Combinational::Adder(_))).count();
    let muxes = components.iter().filter(|c| matches!(c, Combinational::Mux(_))).count();
    assert_eq!(nands, 129);
    assert_eq!(adders, 16);
    assert_eq!(muxes, 9);
}


#[test]
fn alu_graph() {
    let chip = ALU::chip();
    assert_eq!(
        print_graph(&chip),
        "ALU:\n\
         mux_0.a0[0..15] <- x[0..15]\n\
         mux_0.a1[0..15] <- 0\n\
         mux_0.sel <- zx\n\
         not16_1.a[0..15] <- mux_0.out[0..15]\n\
         mux_2.a0[0..15] <- mux_0.out[0..15]\n\
         mux_2.a1[0..15] <- not16_1.out[0..15]\n\
         mux_2.sel <- nx\n\
         mux_3.a0[0..15] <- y[0..15]\n\
         mux_3.a1[0..15] <- 0\n\
         mux_3.sel <- zy\n\
         not16_4.a[0..15] <- mux_3.out[0..15]\n\
         mux_5.a0[0..15] <- mux_3.out[0..15]\n\
         mux_5.a1[0..15] <- not16_4.out[0..15]\n\
         mux_5.sel <- ny\n\
         and16_6.a[0..15] <- mux_2.out[0..15]\n\
         and16_6.b[0..15] <- mux_5.out[0..15]\n\
         not_7.a <- disable\n\
         and_8.a <- f\n\
         and_8.b <- not_7.out\n\
         mux_9.a0[0..15] <- 0\n\
         mux_9.a1[0..15] <- mux_2.out[0..15]\n\
         mux_9.sel <- and_8.out\n\
         mux_10.a0[0..15] <- 0\n\
         mux_10.a1[0..15] <- mux_5.out[0..15]\n\
         mux_10.sel <- and_8.out\n\
         add16_11.a[0..15] <- mux_9.out[0..15]\n\
         add16_11.b[0..15] <- mux_10.out[0..15]\n\
         mux_12.a0[0..15] <- and16_6.out[0..15]\n\
         mux_12.a1[0..15] <- add16_11.out[0..15]\n\
         mux_12.sel <- f\n\
         not16_13.a[0..15] <- mux_12.out[0..15]\n\
         mux_14.a0[0..15] <- mux_12.out[0..15]\n\
         mux_14.a1[0..15] <- not16_13.out[0..15]\n\
         mux_14.sel <- no\n\
         zero16_15.a[0..15] <- mux_14.out[0..15]\n\
         mux_16.a0[0..15] <- mux_14.out[0..15]\n\
         mux_16.a1[0..15] <- 0\n\
         mux_16.sel <- disable\n\
         mux_17.a0 <- zero16_15.out\n\
         mux_17.a1 <- 1\n\
         mux_17.sel <- disable\n\
         neg16_18.a[0..15] <- mux_16.out[0..15]\n\
         ng <- neg16_18.out\n\
         out[0..15] <- mux_16.out[0..15]\n\
         zr <- mux_17.out");
}
