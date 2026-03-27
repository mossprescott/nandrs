/// Alternate Hack CPU implementation, using only an 8-bit ALU.
///
/// This design uses ~20% fewer gates, but requires 2 cycles to execute each Hack instruction.
/// Mostly, it's a test case for simulating an alternative architecure; this one shares no
/// components with the standard CPU beyond the primitives and single-bit logic.
use assignments::project_01::{And, Mux, Nand, Not};
use assignments::project_02::FullAdder;
use assignments::{project_02, project_05};
use simulator::component::Buffer;
use simulator::component::Computational16;
use simulator::declare::BusRef;
use simulator::nat::{N8, N16};
use simulator::{
    Chip, Component, IC, Input, Input1, Input16, Interface, Output, Output16, OutputBus, Reflect,
    expand, fixed,
};

type Input8 = Input<N8>;
type Output8 = OutputBus<N8>;

/// Selects between two 8-bit inputs bit-by-bit, using a single sel bit.
#[derive(Clone, Reflect, Chip)]
pub struct Mux8 {
    pub a0: Input8,
    pub a1: Input8,
    pub sel: Input1,
    pub out: Output8,
}
impl Component for Mux8 {
    type Target = project_02::Project02Component;

    expand! { |this| {
        not_sel: Not { a: this.sel, out: Output::new() },
        for i in 0..8 {
            nand0: Nand { a: not_sel.out.clone().into(), b: this.a0.bit(i),           out: Output::new() },
            nand1: Nand { a: this.sel,                   b: this.a1.bit(i),           out: Output::new() },
            _out:  Nand { a: nand0.out.clone().into(),   b: nand1.out.clone().into(), out: this.out.bit(i) }
        }
    }}
}

/// Inverts each bit of an 8-bit input.
#[derive(Clone, Reflect, Chip)]
pub struct Not8 {
    pub a: Input8,
    pub out: Output8,
}
impl Component for Not8 {
    type Target = project_02::Project02Component;

    expand! { |this| {
        for i in 0..8 {
            _not: Not { a: this.a.bit(i), out: this.out.bit(i) }
        }
    }}
}

/// Bitwise `And` across two 8-bit inputs.
#[derive(Clone, Reflect, Chip)]
pub struct And8 {
    pub a: Input8,
    pub b: Input8,
    pub out: Output8,
}
impl Component for And8 {
    type Target = project_02::Project02Component;

    expand! { |this| {
        for i in 0..8 {
            _and: And { a: this.a.bit(i), b: this.b.bit(i), out: this.out.bit(i) }
        }
    }}
}

/// out = a + b + carry_in (8-bit, with carry out)
#[derive(Clone, Reflect, Chip)]
pub struct Add8 {
    pub a: Input8,
    pub b: Input8,
    pub carry_in: Input1,
    pub out: Output8,
    pub carry_out: Output,
}
impl Component for Add8 {
    type Target = project_02::Project02Component;

    expand! { |this| {
        _carry_out: (0..8).fold(this.carry_in, |carry, i| {
            add: FullAdder {
                a: this.a.bit(i),
                b: this.b.bit(i),
                c: carry,
                sum: this.out.bit(i),
                carry: if i == 7 { this.carry_out } else { Output::new() },
            },
            add.carry.into()
        }),
    }}
}

/// 8-way NAND: AND-tree of all 8 input bits, then invert. Used by Zero8 for efficient simulation
/// (the simulator can coalesce this into a single native ManyWayAnd operation).
#[derive(Clone, Reflect, Chip)]
pub struct Nand8Way {
    pub a: Input8,
    pub out: Output,
}
impl Component for Nand8Way {
    type Target = project_02::Project02Component;

    expand! { |this| {
        // Level 1: pair up adjacent bits
        and_01: And { a: this.a.bit(0).into(), b: this.a.bit(1).into(), out: Output::new() },
        and_23: And { a: this.a.bit(2).into(), b: this.a.bit(3).into(), out: Output::new() },
        and_45: And { a: this.a.bit(4).into(), b: this.a.bit(5).into(), out: Output::new() },
        and_67: And { a: this.a.bit(6).into(), b: this.a.bit(7).into(), out: Output::new() },
        // Level 2
        and_0123: And { a: and_01.out.into(), b: and_23.out.into(), out: Output::new() },
        and_4567: And { a: and_45.out.into(), b: and_67.out.into(), out: Output::new() },
        // Level 3
        and_all: And { a: and_0123.out.into(), b: and_4567.out.into(), out: Output::new() },

        _not: Not { a: and_all.out.into(), out: this.out },
    }}
}

/// Returns 1 if all bits of the 8-bit input are 0.
#[derive(Clone, Reflect, Chip)]
pub struct Zero8 {
    pub a: Input8,
    pub out: Output,
}
impl Component for Zero8 {
    type Target = Combinational8;

    // zero = (!a[0]) & (!a[1]) & ... & (!a[7])
    expand! { |this| {
        // Negate into a single bus; the simulator makes this parallel.
        not: Not8 { a: this.a, out: Output8::new() },

        // Compare them all at once, as if 8-way fan-in was a thing. The simulator
        // handles this efficiently, too.
        nand_all: Nand8Way { a: not.out.into(), out: Output::new() },

        _f: Not { a: nand_all.out.into(), out: this.out },
    }}
}

/// out = true if the most-significant bit of a is 1 (i.e., input is negative in two's complement).
#[derive(Clone, Reflect, Chip)]
pub struct Neg8 {
    pub a: Input8,
    pub out: Output,
}
impl Component for Neg8 {
    type Target = project_02::Project02Component;

    expand! { |this| {
        _sign: Buffer { a: this.a.bit(7), out: this.out },
    }}
}

/// 8-bit ALU, which handles one-half word in a single cycle.
#[derive(Clone, Reflect, Chip)]
pub struct ALU {
    /// Disable: when 1, all outputs are forced to 0. This makes it easier for the simulator to
    /// identify this logic as inactive and avoid spending time evaluating it.
    pub disable: Input1,

    /// "Left" input
    pub x: Input8,
    // "Right" input
    pub y: Input8,

    pub carry_in: Input1,

    /// Zero the x input
    pub zx: Input1,
    /// Negate the x input (i.e. "not")
    pub nx: Input1,
    /// Zero the y input
    pub zy: Input1,
    /// Negate the y input (i.e. "not")
    pub ny: Input1,
    /// 0 => x && y; 1 => x + y
    pub f: Input1,
    /// Negate the result (i.e. "not")
    pub no: Input1,

    /// 8-bit result
    pub out: Output8,
    /// Flag: is the result equal to zero (all bits zero)
    pub zr: Output,
    /// Flag: is the result < 0? (high bit set)
    pub ng: Output,

    /// Carry (overflow) bit from the addition operation, if appl.
    pub carry_out: Output,
}

impl Component for ALU {
    type Target = Combinational8;

    expand! { |this| {
        // zx/nx: conditionally zero then negate x
         x1: Mux8 { sel: this.zx, a0: this.x, a1: fixed(0), out: Output8::new() },
         x2_not: Not8 { a: x1.out.into(), out: Output8::new() },
         x2: Mux8 { sel: this.nx, a0: x1.out.into(), a1: x2_not.out.into(), out: Output8::new() },

         // zy/ny: conditionally zero then negate y
         y1: Mux8 { sel: this.zy, a0: this.y, a1: fixed(0), out: Output8::new() },
         y2_not: Not8 { a: y1.out.into(), out: Output8::new() },
         y2: Mux8 { sel: this.ny, a0: y1.out.into(), a1: y2_not.out.into(), out: Output8::new() },

         and: And8 { a: x2.out.into(), b: y2.out.into(), out: Output8::new() },

         // Gate Add8 inputs: only active when f=1 AND !disable. Mux8 gives Add8
         // its own copy of the inputs, breaking the sharing with And8 so the nesting
         // algorithm can move all Add8 nands into a mux branch.
         not_disable: Not { a: this.disable, out: Output::new() },
         add_active: And { a: this.f, b: not_disable.out.into(), out: Output::new() },
         add_x: Mux8 { sel: add_active.out.into(), a0: fixed(0), a1: x2.out.into(), out: Output8::new() },
         add_y: Mux8 { sel: add_active.out.into(), a0: fixed(0), a1: y2.out.into(), out: Output8::new() },
         add: Add8 { a: add_x.out.into(), b: add_y.out.into(), carry_in: this.carry_in, out: Output8::new(), carry_out: this.carry_out },

         result: Mux8 { sel: this.f, a0: and.out.into(), a1: add.out.into(), out: Output8::new() },
         rn: Not8 { a: result.out.into(), out: Output8::new() },
         raw_out: Mux8 { sel: this.no, a0: result.out.into(), a1: rn.out.into(), out: Output8::new() },

         // Compute zr from raw_out (before the disable gate) so it can be nested
         // into the disable mux branch along with the rest of the ALU chain.
         raw_zr: Zero8 { a: raw_out.out.into(), out: Output::new() },

         // Gate output and zr with disable.  When disabled: out=0, zr=1.
         out_gate: Mux8 { sel: this.disable, a0: raw_out.out.into(), a1: fixed(0), out: this.out },
         zr_gate: Mux { sel: this.disable, a0: raw_zr.out.into(), a1: fixed(1), out: this.zr },

         // ng reads from the gated output; when disabled out=0 so ng=0 (correct).
         // Neg8 is 0 nands (just a buffer) so there's nothing to skip.
         rneg: Neg8 { a: this.out.into(), out: this.ng },
    }}
}

#[derive(Clone, Reflect, Component)]
pub enum Combinational8 {
    #[delegate]
    Project02(project_02::Project02Component),
    Mux8(Mux8),
    Not8(Not8),
    And8(And8),
    Add8(Add8),
    Nand8Way(Nand8Way),
    Zero8(Zero8),
    Neg8(Neg8),
    ALU(ALU),
}

use simulator::component::Combinational;

/// Recursively expand until only Nands and Buffers are left.
pub fn flatten_to_nands<C: Reflect + Component<Target = Combinational8>>(
    chip: C,
) -> IC<Combinational> {
    fn go(comp: Combinational8) -> Vec<Combinational> {
        match comp {
            Combinational8::Project02(p) => project_02::flatten(p).components,
            other => match other.expand() {
                Some(ic) => ic.components.into_iter().flat_map(go).collect(),
                None => panic!("Did not reduce to primitive: {:?}", other.name()),
            },
        }
    }
    IC {
        name: format!("{} (flat)", chip.name()),
        intf: chip.reflect(),
        components: chip
            .expand()
            .expect("flatten_to_nands() requires a non-primitive component")
            .components
            .into_iter()
            .flat_map(go)
            .collect(),
    }
}

#[derive(Clone, Reflect, Chip)]
pub struct PC {
    /// Reset to zero on the next cycle
    pub reset: Input1,

    /// Load an arbitrary address
    pub addr: Input16,
    pub load: Input1,

    /// Increment to point to the next address on the next cycle
    pub inc: Input1,

    pub out: Output16,
}

#[derive(Clone, Reflect, Chip)]
pub struct CPU {
    /// Return to a known state (i.e. jump to address 0)
    pub reset: Input1,

    /// Address of the next instruction to load
    pub pc: Output16,

    /// The bits of the current instruction
    pub instr: Input16,

    pub mem_data_out: Output16,
    pub mem_write: Output,

    /// Feed-forward: address to write at the end of this cycle, and read from in the *next* cycle
    pub mem_addr: Output16,

    pub mem_data_in: Input16,
}

#[derive(Clone, Reflect, Chip)]
pub struct Computer {
    /// A way to force the CPU to return to a known state (i.e. jump to address 0)
    pub reset: Input1,

    /// Useful for debugging, but also acts as a root for traversing the graph
    pub pc: Output16,
}

#[derive(Reflect, Component)]
pub enum EightComponent {
    #[delegate]
    Project05(project_05::Project05Component),
    Combinational8(Combinational8),
    // PC(PC),
    // CPU(CPU),
    // Computer(Computer),
}

/// Recursively expand until only Nands, Registers, RAMs, ROMs, and MemorySystems are left.
pub fn flatten<C: Reflect + Into<EightComponent>>(chip: C) -> IC<Computational16> {
    fn go(comp: EightComponent) -> Vec<Computational16> {
        match comp.expand() {
            None => match comp {
                EightComponent::Project05(p) => project_05::flatten(p).components,
                _ => panic!("Did not reduce to primitive: {:?}", comp.name()),
            },
            Some(ic) => ic.components.into_iter().flat_map(go).collect(),
        }
    }
    IC {
        name: format!("{} (flat)", chip.name()),
        intf: chip.reflect(),
        components: go(chip.into()),
    }
}

/// Like `flatten`, but uses native Mux/Adder components for efficient simulation.
pub fn flatten_for_simulation<C: Reflect + Into<EightComponent>>(
    chip: C,
) -> IC<simulator::component::native::Simulational<N16, N16>> {
    use simulator::component::native::Simulational;
    fn go(comp: EightComponent) -> Vec<Simulational<N16, N16>> {
        // Delegate Project05 subtrees immediately, so their interception logic handles Mux/Adder:
        if let EightComponent::Project05(p) = comp {
            return project_05::flatten_for_simulation(p).components;
        }
        match comp.expand() {
            Some(ic) => ic.components.into_iter().flat_map(go).collect(),
            None => panic!("Did not reduce to primitive: {:?}", comp.name()),
        }
    }
    IC {
        name: format!("{} (flat/sim)", chip.name()),
        intf: chip.reflect(),
        components: go(chip.into()),
    }
}

#[cfg(test)]
mod test {
    use std::collections::HashMap;

    use crate::computer::{ALU, flatten_to_nands};
    use simulator::component::{Combinational, count_combinational};
    use simulator::nat::N16;
    use simulator::word::Word;
    use simulator::{Chip as _, eval, print_graph};

    // Note: the ALU and related components are all 8-bit, but end up embedded in a 16-bit circuit, so for simplicity,
    // treat values as 16-bits
    fn eval16<'a>(
        chip: &simulator::IC<Combinational>,
        inputs: impl IntoIterator<Item = (&'a str, Word<N16>)>,
    ) -> HashMap<String, Word<N16>> {
        eval(chip, inputs)
    }

    #[test]
    fn alu_truth_table() {
        let chip = ALU::chip();

        // When it breaks, it's nice to see what it tried to do
        print!("{}", print_graph(&chip));

        let chip = flatten_to_nands(chip);

        // 0 = 0 + 0
        let r = eval16(
            &chip,
            [
                ("x", 0u16.into()),
                ("y", 0u16.into()),
                ("carry_in", false.into()),
                ("zx", true.into()),
                ("nx", false.into()),
                ("zy", true.into()),
                ("ny", false.into()),
                ("f", true.into()),
                ("no", false.into()),
            ],
        );
        assert_eq!(r["out"].unsigned(), 0);
        assert_eq!(r["zr"].unsigned(), 1);
        assert_eq!(r["ng"].unsigned(), 0); // 0

        // 1 = !(-1 + -1)
        let r = eval16(
            &chip,
            [
                ("x", 0u16.into()),
                ("y", 0u16.into()),
                ("carry_in", false.into()),
                ("zx", true.into()),
                ("nx", true.into()),
                ("zy", true.into()),
                ("ny", true.into()),
                ("f", true.into()),
                ("no", true.into()),
            ],
        );
        assert_eq!(r["out"].unsigned(), 1);
        assert_eq!(r["zr"].unsigned(), 0);
        assert_eq!(r["ng"].unsigned(), 0); // 1

        // -1 = -1 + 0
        let r = eval16(
            &chip,
            [
                ("x", 0u16.into()),
                ("y", 0u16.into()),
                ("carry_in", false.into()),
                ("zx", true.into()),
                ("nx", true.into()),
                ("zy", true.into()),
                ("ny", false.into()),
                ("f", true.into()),
                ("no", false.into()),
            ],
        );
        assert_eq!(r["out"].unsigned(), 0xff);
        assert_eq!(r["zr"].unsigned(), 0);
        assert_eq!(r["ng"].unsigned(), 1); // -1
        assert_eq!(r["carry_out"].unsigned(), 0);

        // x = x and 0xfff
        let r = eval16(
            &chip,
            [
                ("x", 5u16.into()),
                ("y", 3u16.into()),
                ("carry_in", false.into()),
                ("zx", false.into()),
                ("nx", false.into()),
                ("zy", true.into()),
                ("ny", true.into()),
                ("f", false.into()),
                ("no", false.into()),
            ],
        );
        assert_eq!(r["out"].unsigned(), 5);
        assert_eq!(r["zr"].unsigned(), 0);
        assert_eq!(r["ng"].unsigned(), 0); // x

        // y = 0xfff and y
        let r = eval16(
            &chip,
            [
                ("x", 5u16.into()),
                ("y", 3u16.into()),
                ("carry_in", false.into()),
                ("zx", true.into()),
                ("nx", true.into()),
                ("zy", false.into()),
                ("ny", false.into()),
                ("f", false.into()),
                ("no", false.into()),
            ],
        );
        assert_eq!(r["out"].unsigned(), 3);
        assert_eq!(r["zr"].unsigned(), 0);
        assert_eq!(r["ng"].unsigned(), 0); // y

        // x + y
        let r = eval16(
            &chip,
            [
                ("x", 5u16.into()),
                ("y", 3u16.into()),
                ("carry_in", false.into()),
                ("zx", false.into()),
                ("nx", false.into()),
                ("zy", false.into()),
                ("ny", false.into()),
                ("f", true.into()),
                ("no", false.into()),
            ],
        );
        assert_eq!(r["out"].unsigned(), 8);
        assert_eq!(r["zr"].unsigned(), 0);
        assert_eq!(r["ng"].unsigned(), 0); // x + y
        assert_eq!(r["carry_out"].unsigned(), 0);

        // x - y = !(!x + y)
        let r = eval16(
            &chip,
            [
                ("x", 5u16.into()),
                ("y", 3u16.into()),
                ("carry_in", false.into()),
                ("zx", false.into()),
                ("nx", true.into()),
                ("zy", false.into()),
                ("ny", false.into()),
                ("f", true.into()),
                ("no", true.into()),
            ],
        );
        assert_eq!(r["out"].unsigned(), 2);
        assert_eq!(r["zr"].unsigned(), 0);
        assert_eq!(r["ng"].unsigned(), 0); // x - y
        assert_eq!(r["carry_out"].unsigned(), 0);

        // x and y
        let r = eval16(
            &chip,
            [
                ("x", 0b1010u16.into()),
                ("y", 0b1100u16.into()),
                ("carry_in", false.into()),
                ("zx", false.into()),
                ("nx", false.into()),
                ("zy", false.into()),
                ("ny", false.into()),
                ("f", false.into()),
                ("no", false.into()),
            ],
        );
        assert_eq!(r["out"].unsigned(), 0b1000);
        assert_eq!(r["zr"].unsigned(), 0);
        assert_eq!(r["ng"].unsigned(), 0); // x AND y

        // x or y = !(!x and !y)
        let r = eval16(
            &chip,
            [
                ("x", 0b1010u16.into()),
                ("y", 0b0101u16.into()),
                ("carry_in", false.into()),
                ("zx", false.into()),
                ("nx", true.into()),
                ("zy", false.into()),
                ("ny", true.into()),
                ("f", false.into()),
                ("no", true.into()),
            ],
        );
        assert_eq!(r["out"].unsigned(), 0b1111);
        assert_eq!(r["zr"].unsigned(), 0);
        assert_eq!(r["ng"].unsigned(), 0); // x OR y

        // x + y + 1 (carry from previous cycle)
        let r = eval16(
            &chip,
            [
                ("x", 5u16.into()),
                ("y", 3u16.into()),
                ("carry_in", true.into()),
                ("zx", false.into()),
                ("nx", false.into()),
                ("zy", false.into()),
                ("ny", false.into()),
                ("f", true.into()),
                ("no", false.into()),
            ],
        );
        assert_eq!(r["out"].unsigned(), 9);
        assert_eq!(r["zr"].unsigned(), 0);
        assert_eq!(r["ng"].unsigned(), 0);
        assert_eq!(r["carry_out"].unsigned(), 0);

        // x + y (and carry to next cycle)
        let r = eval16(
            &chip,
            [
                ("x", 128u16.into()),
                ("y", 128u16.into()),
                ("carry_in", false.into()),
                ("zx", false.into()),
                ("nx", false.into()),
                ("zy", false.into()),
                ("ny", false.into()),
                ("f", true.into()),
                ("no", false.into()),
            ],
        );
        assert_eq!(r["out"].unsigned(), 0); // low-half-word of 256
        assert_eq!(r["zr"].unsigned(), 1);
        assert_eq!(r["ng"].unsigned(), 0);
        assert_eq!(r["carry_out"].unsigned(), 1);
    }

    #[test]
    fn alu_optimal() {
        let chip = flatten_to_nands(ALU::chip());
        assert_eq!(count_combinational(&chip.components).nands, 368); // Compare to 720
    }
}
