//! Alternate Hack CPU implementation, using only 8-bit registers and adders.
//!
//! This design uses ~20% fewer gates, but requires 2 cycles to execute each Hack instruction.
//! Mostly, it's a test case for simulating an alternative architecure; this one shares no
//! components with the standard CPU beyond the primitives and single-bit logic.
use assignments::project_01::{And, Mux, Nand, Not, Or};
use assignments::project_02::{FullAdder, HalfAdder};
use assignments::project_05::Decode;
use frunk::coproduct::CoprodInjector;
use frunk::{Coprod, hlist};
use simulator::{Input, OutputBus};
use simulator::component::{Buffer, Computational, DFF, native};
use simulator::component::{Combinational, Computational16, MemorySystem16, ROM16};
use simulator::nat::{N8, N16};
use simulator::{
    Chip, Flat, IC, Input1, Input16, Interface, Output, Output16, Reflect, declare::BusRef, expand,
    fixed, flatten_g,
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
impl Mux8 {
    expand!([Not, Nand], |this| {
        not_sel: Not { a: this.sel, out: Output::new() },
        for i in 0..8 {
            nand0: Nand { a: not_sel.out.clone().into(), b: this.a0.bit(i),           out: Output::new() },
            nand1: Nand { a: this.sel,                   b: this.a1.bit(i),           out: Output::new() },
            _out:  Nand { a: nand0.out.clone().into(),   b: nand1.out.clone().into(), out: this.out.bit(i) }
        }
    });
}

/// Inverts each bit of an 8-bit input.
#[derive(Clone, Reflect, Chip)]
pub struct Not8 {
    pub a: Input8,
    pub out: Output8,
}
impl Not8 {
    expand!([Not], |this| {
        for i in 0..8 {
            _not: Not { a: this.a.bit(i), out: this.out.bit(i) }
        }
    });
}

/// Bitwise `And` across two 8-bit inputs.
#[derive(Clone, Reflect, Chip)]
pub struct And8 {
    pub a: Input8,
    pub b: Input8,
    pub out: Output8,
}
impl And8 {
    expand!([And], |this| {
        for i in 0..8 {
            _and: And { a: this.a.bit(i), b: this.b.bit(i), out: this.out.bit(i) }
        }
    });
}

/// out = a + carry_in (8-bit, with carry out)
#[derive(Clone, Reflect, Chip)]
pub struct Inc8 {
    pub a: Input8,
    pub carry_in: Input1,
    pub out: Output8,
    pub carry_out: Output,
}
impl Inc8 {
    expand!([HalfAdder], |this| {
        _carry_out: (0..8).fold(this.carry_in, |carry, i| {
            add: HalfAdder {
                a: this.a.bit(i),
                b: carry,
                sum: this.out.bit(i),
                carry: if i == 7 { this.carry_out } else { Output::new() },
            },
            add.carry.into()
        }),
    });
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
impl Add8 {
    expand!([FullAdder], |this| {
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
    });
}

/// 8-way NAND: AND-tree of all 8 input bits, then invert. Used by Zero8 for efficient simulation
/// (the simulator can coalesce this into a single native ManyWayAnd operation).
#[derive(Clone, Reflect, Chip)]
pub struct Nand8Way {
    pub a: Input8,
    pub out: Output,
}
impl Nand8Way {
    expand!([And, Not], |this| {
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
    });
}

/// Returns 1 if all bits of the 8-bit input are 0.
#[derive(Clone, Reflect, Chip)]
pub struct Zero8 {
    pub a: Input8,
    pub out: Output,
}
impl Zero8 {
    expand!([Not8, Nand8Way, Not], |this| {
        // Negate into a single bus; the simulator makes this parallel.
        not: Not8 { a: this.a, out: Output8::new() },

        // Compare them all at once, as if 8-way fan-in was a thing. The simulator
        // handles this efficiently, too.
        nand_all: Nand8Way { a: not.out.into(), out: Output::new() },

        _f: Not { a: nand_all.out.into(), out: this.out },
    });
}

/// out = true if the most-significant bit of a is 1 (i.e., input is negative in two's complement).
#[derive(Clone, Reflect, Chip)]
pub struct Neg8 {
    pub a: Input8,
    pub out: Output,
}
impl Neg8 {
    expand!([Buffer], |this| {
        _sign: Buffer { a: this.a.bit(7), out: this.out },
    });
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

impl ALU {
    expand!([Mux8, Not8, And8, Add8, Zero8, Neg8, Not, And, Mux], |this| {
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
    });
}

/// Slice a 16-bit bus into high and low half-words.
#[derive(Clone, Reflect, Chip)]
pub struct Split {
    pub a: Input16,

    pub hi: Output8,
    pub lo: Output8,
}

impl Split {
    expand!([Buffer], |this| {
        for i in 0..8 {
            _lo: Buffer { a: this.a.bit(i), out: this.lo.bit(i) },
            _hi: Buffer { a: this.a.bit(8+i), out: this.hi.bit(i) },
        }
    });
}

/// Assemble high and low half-words into a 16-bit signal.
#[derive(Clone, Reflect, Chip)]
pub struct Join {
    pub hi: Input8,
    pub lo: Input8,

    pub out: Output16,
}

impl Join {
    expand!([Buffer], |this| {
        for i in 0..8 {
            _lo: Buffer { a: this.lo.bit(i), out: this.out.bit(i) },
            _hi: Buffer { a: this.hi.bit(i), out: this.out.bit(8+i) },
        }
    });
}

pub type Combinational8T = Coprod!(
    Nand, Buffer, Not, And, Or, Mux, HalfAdder, FullAdder, Mux8, Not8, And8, Inc8, Add8, Nand8Way,
    Zero8, Neg8, ALU, Split, Join, Decode
);

/// Recursively expand until only Nands and Buffers are left (combinational only).
pub fn flatten_to_nands<C, Idx>(chip: C) -> IC<Combinational>
where
    C: Reflect,
    Combinational8T: CoprodInjector<C, Idx>,
{
    flatten_g::<C, Combinational8T, Idx, Combinational, _>(
        chip,
        "flat",
        hlist![
            |c: Nand| Flat::Done(vec![Combinational::Nand(c)]),
            |c: Buffer| Flat::Done(vec![Combinational::Buffer(c)]),
            |c: Not| Flat::Continue(c.expand()),
            |c: And| Flat::Continue(c.expand()),
            |c: Or| Flat::Continue(c.expand()),
            |c: Mux| Flat::Continue(c.expand()),
            |c: HalfAdder| Flat::Continue(c.expand()),
            |c: FullAdder| Flat::Continue(c.expand()),
            |c: Mux8| Flat::Continue(c.expand()),
            |c: Not8| Flat::Continue(c.expand()),
            |c: And8| Flat::Continue(c.expand()),
            |c: Inc8| Flat::Continue(c.expand()),
            |c: Add8| Flat::Continue(c.expand()),
            |c: Nand8Way| Flat::Continue(c.expand()),
            |c: Zero8| Flat::Continue(c.expand()),
            |c: Neg8| Flat::Continue(c.expand()),
            |c: ALU| Flat::Continue(c.expand()),
            |c: Split| Flat::Continue(c.expand()),
            |c: Join| Flat::Continue(c.expand()),
            |c: Decode| Flat::Continue(c.expand()),
        ],
    )
}


/// 8-bit wide register made out of DFFs and a Mux8 for the write-enable.
#[derive(Clone, Reflect, Chip)]
pub struct Register8 {
    pub data_in: Input8,
    pub write: Input1,
    pub data_out: Output8,
}

impl Register8 {
    expand!([Mux8, DFF], |this| {
        next: Mux8 { a0: this.data_out.into(), a1: this.data_in, sel: this.write, out: Output8::new() },
        for i in 0..8 {
            dff: DFF { a: next.out.bit(i).into(), out: this.data_out.bit(i) },
        }
    });
}

/// An 8-bit latch which is just 8 DFFs; rewritten with a "native" register for fast simulation.
#[derive(Clone, Reflect, Chip)]
pub struct DFF8 {
    pub a: Input8,
    pub out: Output8,
}
impl DFF8 {
    expand!([DFF], |this| {
        for i in 0..8 {
            dff: DFF { a: this.a.bit(i).into(), out: this.out.bit(i) },
        }
    });
}


/// PC maintaining a 16-bit instruction address which is actually stored in a pair of 8-bit
/// registers, using an 8-bit increment unit to compute the new address across 2 cycles when `inc`
/// is asserted.
///
/// In the first cycle (`top-half` high), the low 8 bits are incremented and the result is latched.
/// In the second cycle (`bottom_half` high), the high 8 bits are incremented (if a one was carried
/// out from the low-half-word Inc), and the two registers are updated with new values depending on
/// the control signals.
#[derive(Clone, Reflect, Chip)]
pub struct PC {
    /// True in the first of each pair of cycles.
    pub top_half: Input1,
    /// True in the second of each pair of cycles.
    pub bottom_half: Input1,

    /// Reset to zero on the next cycle
    pub reset: Input1,

    /// Load an arbitrary address
    pub addr: Input16,
    pub load: Input1,

    /// Increment to point to the next address on the next cycle
    pub inc: Input1,

    pub out: Output16,
}

impl PC {
    expand!([Mux8, Not, And, Or, Inc8, Split, Join, Register8, DFF8], |this| {
        lo_out: forward Output8::new(),
        hi_out: forward Output8::new(),
        latch_out: forward Output8::new(),

        inc_src: Mux8 { a0: lo_out.into(), a1: hi_out.into(), sel: this.bottom_half, out: Output8::new() },
        // carry_in: if top_half then 1 else (lo_out.bit(7) and !latch_out.bit(7))
        is_low: Not { a: latch_out.bit(7).into(), out: Output::new() },
        dropped: And { a: lo_out.bit(7).into(), b: is_low.out.into(), out: Output::new() },
        carry_in: Or { a: this.top_half, b: dropped.out.into(), out: Output::new() },
        inc: Inc8 { a: inc_src.out.into(), carry_in: carry_in.out.into(), out: Output8::new(), carry_out: Output::new() },

        next0_lo: Mux8 { a0: lo_out.into(), a1: latch_out.into(), sel: this.inc, out: Output8::new() },
        next0_hi: Mux8 { a0: hi_out.into(), a1: inc.out.into(), sel: this.inc, out: Output8::new() },

        addr_split: Split { a: this.addr, lo: Output8::new(), hi: Output8::new() },
        next1_lo: Mux8 { a0: next0_lo.out.into(), a1: addr_split.lo.into(), sel: this.load, out: Output8::new() },
        next1_hi: Mux8 { a0: next0_hi.out.into(), a1: addr_split.hi.into(), sel: this.load, out: Output8::new() },

        next2_lo: Mux8 { a0: next1_lo.out.into(), a1: fixed(0), sel: this.reset, out: Output8::new() },
        next2_hi: Mux8 { a0: next1_hi.out.into(), a1: fixed(0), sel: this.reset, out: Output8::new() },

        out: Join { lo: lo_out.into(), hi: hi_out.into(), out: this.out },

        // Note: reset is effective in any (single) cycle; others only in bottom_half
        write: Or { a: this.bottom_half, b: this.reset, out: Output::new() },
        lo: Register8 { data_in: next2_lo.out.into(), write: write.out.into(), data_out: lo_out },
        hi: Register8 { data_in: next2_hi.out.into(), write: write.out.into(), data_out: hi_out },

        // Latch Inc result for next cycle.
        latch: DFF8 { a: inc.out.into(), out: latch_out },
    });
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

impl CPU {
    expand!([Decode, Mux8, Or, And, Nand, Not, ALU, Split, Join, PC, DFF8, DFF, Register8, Mux], |this| {
        top_half: forward Output::new(),
        bottom_half: forward Output::new(),

        alu_latch_out: forward Output8::new(),
        zr_latch_out: forward Output::new(),
        carry_latch_out: forward Output::new(),

        reg_a_lo_out: forward Output8::new(),
        reg_a_hi_out: forward Output8::new(),

        reg_d_lo_out: forward Output8::new(),
        reg_d_hi_out: forward Output8::new(),

        decode: Decode {
            instr: this.instr,

            is_c: Output::new(),
            is_a: Output::new(),

            read_m: Output::new(),

            zx: Output::new(), nx: Output::new(),
            zy: Output::new(), ny: Output::new(),
            f:  Output::new(), no: Output::new(),

            write_a: Output::new(), write_m: Output::new(), write_d: Output::new(),

            jmp_lt:  Output::new(), jmp_eq:  Output::new(), jmp_gt:  Output::new(),
        },

        x_src: Mux8 { a0: reg_d_lo_out.into(), a1: reg_d_hi_out.into(), sel: bottom_half.into(), out: Output8::new() },

        // === load_a = is_a OR write_a, gated to bottom_half cycle ===
        will_load_a: Or { a: decode.is_a.into(), b: decode.write_a.into(), out: Output::new() },
        load_a: And { a: will_load_a.out.into(), b: bottom_half.into(), out: Output::new() },

        // === ALU Y mux: sel=read_m → a0=A, a1=mem_in ===
        reg_a_sel: Mux8 { a0: reg_a_lo_out.into(), a1: reg_a_hi_out.into(), sel: bottom_half.into(), out: Output8::new() },
        mem_data_in: Split { a: this.mem_data_in, lo: Output8::new(), hi: Output8::new() },
        mem_data_sel: Mux8 { a0: mem_data_in.lo.into(), a1: mem_data_in.hi.into(), sel: bottom_half.into(), out: Output8::new() },
        y_src: Mux8 {
            a0:  reg_a_sel.out.into(),
            a1:  mem_data_sel.out.into(),
            sel: decode.read_m.into(),
            out: Output8::new(),
        },

        carry_in: And { a: carry_latch_out.into(), b: bottom_half.into(), out: Output::new() },

        alu: ALU {
            x:   x_src.out.into(),
            y:   y_src.out.into(),
            carry_in: carry_in.out.into(),
            zx:  decode.zx.into(), nx: decode.nx.into(),
            zy:  decode.zy.into(), ny: decode.ny.into(),
            f:   decode.f.into(),  no: decode.no.into(),
            disable: decode.is_a.into(),
            out: Output8::new(),
            zr:  Output::new(),
            ng:  Output::new(),
            carry_out: Output::new(),
        },

        mem_data_out: Join { lo: alu_latch_out.into(), hi: alu.out.into(), out: this.mem_data_out },


        // // === A register data mux: AFTER ALU ===
        // // sel=is_a → a1=instr (A-instr), a0=ALU output (C-instr with dest=A)
        instr: Split { a: this.instr, lo: Output8::new(), hi: Output8::new() },
        a_data_lo: Mux8 { a0: alu_latch_out.into(), a1: instr.lo.into(), sel: decode.is_a.into(), out: Output8::new() },
        a_data_hi: Mux8 { a0: alu.out.into(),       a1: instr.hi.into(), sel: decode.is_a.into(), out: Output8::new() },

        // === next_addr: if A is being written this cycle, expose the new A value as the
        // address for the memory system (so RAM latches the right read address); otherwise
        // expose the current A.out. Write address is always A.out (load_a=0 when write_m=1). ===
        next_addr_lo: Mux8 { a0: reg_a_lo_out.into(), a1: a_data_lo.out.into(), sel: load_a.out.into(), out: Output8::new() },
        next_addr_hi: Mux8 { a0: reg_a_hi_out.into(), a1: a_data_hi.out.into(), sel: load_a.out.into(), out: Output8::new() },
        next_addr: Join { lo: next_addr_lo.out.into(), hi: next_addr_hi.out.into(), out: this.mem_addr },

        // === mem_write (write_m already gated with is_c in Decode) ===
        mem_write: And { a: bottom_half.into(), b: decode.write_m.into(), out: this.mem_write },

        // === Jump logic ===
        not_ng:   Not { a: alu.ng.into(), out: Output::new() },
        not_zr:   Nand { a: zr_latch_out.into(), b: alu.zr.into(), out: Output::new() },
        zr_full:  Not { a: not_zr.out.into(), out: Output::new() },
        is_pos:   And { a: not_ng.out.into(), b: not_zr.out.into(), out: Output::new() },
        // Jump signals already gated with is_c in Decode.
        jlt_and:  And { a: decode.jmp_lt.into(), b: alu.ng.into(), out: Output::new() },
        jeq_and:  And { a: decode.jmp_eq.into(), b: zr_full.out.into(), out: Output::new() },
        jgt_and:  And { a: decode.jmp_gt.into(), b: is_pos.out.into(), out: Output::new() },
        j_lt_eq:  Or  { a: jlt_and.out.into(), b: jeq_and.out.into(), out: Output::new() },
        jump_any: Or  { a: j_lt_eq.out.into(), b: jgt_and.out.into(), out: Output::new() },

        // Gate all the jump logic to bottom_half cycle. Note: this is really just (jump_any && bottom_half), but
        // makes the jump condition calculation explicitly unused on every other cycle.
        jump_any_gate: Mux { a0: fixed(0), a1: jump_any.out.into(), sel: bottom_half.into(), out: Output::new() },

        reg_a_joined: Join { lo: reg_a_lo_out.into(), hi: reg_a_hi_out.into(), out: Output16::new() },

        pc: PC {
            top_half: top_half.into(),
            bottom_half: bottom_half.into(),
            addr:  reg_a_joined.out.into(),
            load:  jump_any_gate.out.into(),
            inc:   fixed(1),
            reset: this.reset.into(),
            out:   this.pc,
        },

        // Finally, all the registers and latches:

        alu_latch: DFF8 { a: alu.out.into(), out: alu_latch_out },
        zr_latch: DFF { a: alu.zr.into(), out: zr_latch_out },
        carry_latch: DFF { a: alu.carry_out.into(), out: carry_latch_out },

        reg_a_lo: Register8 { data_in: a_data_lo.out.into(), write: load_a.out.into(), data_out: reg_a_lo_out },
        reg_a_hi: Register8 { data_in: a_data_hi.out.into(), write: load_a.out.into(), data_out: reg_a_hi_out },

        write_d: And { a: decode.write_d.into(), b: bottom_half.into(), out: Output::new() },
        reg_d_lo: Register8 { data_in: alu_latch_out.into(), write: write_d.out.into(), data_out: reg_d_lo_out },
        reg_d_hi: Register8 { data_in: alu.out.into(),       write: write_d.out.into(), data_out: reg_d_hi_out },

        // Note: reset forces top_half, so only it only has to be asserted for a single cycle
        not_top: Not { a: top_half.into(), out: bottom_half },
        next_cycle: Or { a: not_top.out.into(), b: this.reset, out: Output::new() },
        cycle_dff: DFF { a: next_cycle.out.into(), out: top_half },
    });
}

#[derive(Clone, Reflect, Chip)]
pub struct Computer {
    /// A way to force the CPU to return to a known state (i.e. jump to address 0)
    pub reset: Input1,

    /// Useful for debugging, but also acts as a root for traversing the graph
    pub pc: Output16,
}

impl Computer {
    expand!([ROM16, CPU, MemorySystem16], |this| {
        mem_out: forward Output16::new(),

        rom: ROM16 {
            size: 32 * 1024,
            addr: this.pc.into(),
            out:  Output16::new(),
        },

        cpu: CPU {
            reset:        this.reset,
            pc:           this.pc,
            instr:        rom.out.into(),
            mem_data_out: Output16::new(),
            mem_write:    Output::new(),
            mem_addr:     Output16::new(),
            mem_data_in:  mem_out.into(),
        },

        memory: MemorySystem16 {
            addr:     cpu.mem_addr.into(),
            write:    cpu.mem_write.into(),
            data_in:  cpu.mem_data_out.into(),
            data_out: mem_out,
        },
    });
}

pub type EightComponentT = Coprod!(
    Nand,
    Buffer,
    Not,
    And,
    Or,
    Mux,
    HalfAdder,
    FullAdder,
    Mux8,
    Not8,
    And8,
    Inc8,
    Add8,
    Nand8Way,
    Zero8,
    Neg8,
    ALU,
    Split,
    Join,
    Decode,
    Register8,
    DFF8,
    DFF,
    ROM16,
    MemorySystem16,
    PC,
    CPU,
    Computer
);

/// Recursively expand until only Nands, Registers, RAMs, ROMs, and MemorySystems are left.
pub fn flatten<C, Idx>(chip: C) -> IC<Computational16>
where
    C: Reflect,
    EightComponentT: CoprodInjector<C, Idx>,
{
    flatten_g::<C, EightComponentT, Idx, Computational16, _>(
        chip,
        "flat",
        hlist![
            |c: Nand| Flat::Done(vec![Computational::Nand(c)]),
            |c: Buffer| Flat::Done(vec![Computational::Buffer(c)]),
            |c: Not| Flat::Continue(c.expand()),
            |c: And| Flat::Continue(c.expand()),
            |c: Or| Flat::Continue(c.expand()),
            |c: Mux| Flat::Continue(c.expand()),
            |c: HalfAdder| Flat::Continue(c.expand()),
            |c: FullAdder| Flat::Continue(c.expand()),
            |c: Mux8| Flat::Continue(c.expand()),
            |c: Not8| Flat::Continue(c.expand()),
            |c: And8| Flat::Continue(c.expand()),
            |c: Inc8| Flat::Continue(c.expand()),
            |c: Add8| Flat::Continue(c.expand()),
            |c: Nand8Way| Flat::Continue(c.expand()),
            |c: Zero8| Flat::Continue(c.expand()),
            |c: Neg8| Flat::Continue(c.expand()),
            |c: ALU| Flat::Continue(c.expand()),
            |c: Split| Flat::Continue(c.expand()),
            |c: Join| Flat::Continue(c.expand()),
            |c: Decode| Flat::Continue(c.expand()),
            |c: Register8| Flat::Continue(c.expand()),
            |c: DFF8| Flat::Continue(c.expand()),
            |c: DFF| Flat::Done(vec![Computational::DFF(c)]),
            |c: ROM16| Flat::Done(vec![Computational::ROM(c)]),
            |c: MemorySystem16| Flat::Done(vec![Computational::MemorySystem(c)]),
            |c: PC| Flat::Continue(c.expand()),
            |c: CPU| Flat::Continue(c.expand()),
            |c: Computer| Flat::Continue(c.expand()),
        ],
    )
}

/// Like `flatten`, but uses native Mux/Adder components for efficient simulation.
pub fn flatten_for_simulation<C, Idx>(chip: C) -> IC<native::Simulational<N16, N16>>
where
    C: Reflect,
    EightComponentT: CoprodInjector<C, Idx>,
{
    flatten_g::<C, EightComponentT, Idx, native::Simulational<N16, N16>, _>(
        chip,
        "flat/sim",
        hlist![
            // Delegate project_02 types:
            |c: Nand| Flat::Done(assignments::project_02::flatten_for_simulation(c).components),
            |c: Buffer| Flat::Done(assignments::project_02::flatten_for_simulation(c).components),
            |c: Not| Flat::Done(assignments::project_02::flatten_for_simulation(c).components),
            |c: And| Flat::Done(assignments::project_02::flatten_for_simulation(c).components),
            |c: Or| Flat::Done(assignments::project_02::flatten_for_simulation(c).components),
            |c: Mux| Flat::Done(assignments::project_02::flatten_for_simulation(c).components),
            |c: HalfAdder| Flat::Done(
                assignments::project_02::flatten_for_simulation(c).components
            ),
            |c: FullAdder| Flat::Done(
                assignments::project_02::flatten_for_simulation(c).components
            ),
            // Mux8 → native mux
            |c: Mux8| Flat::Done(vec![
                native::Mux {
                    a0: c.a0,
                    a1: c.a1,
                    sel: c.sel,
                    out: c.out
                }
                .into()
            ]),
            |c: Not8| Flat::Continue(c.expand()),
            |c: And8| Flat::Continue(c.expand()),
            |c: Inc8| Flat::Continue(c.expand()),
            |c: Add8| Flat::Continue(c.expand()),
            |c: Nand8Way| Flat::Continue(c.expand()),
            |c: Zero8| Flat::Continue(c.expand()),
            |c: Neg8| Flat::Continue(c.expand()),
            |c: ALU| Flat::Continue(c.expand()),
            |c: Split| Flat::Continue(c.expand()),
            |c: Join| Flat::Continue(c.expand()),
            |c: Decode| Flat::Continue(c.expand()),
            |c: Register8| Flat::Done(vec![
                native::Register {
                    data_in: c.data_in,
                    write: c.write,
                    data_out: c.data_out,
                }.into()
            ]),
            |c: DFF8| Flat::Done(vec![
                native::Register {
                    data_in: c.a,
                    write: fixed(1),
                    data_out: c.out,
                }.into()
            ]),
            |c: DFF| Flat::Done(vec![Computational::DFF(c).into()]),
            |c: ROM16| Flat::Done(vec![Computational::ROM(c).into()]),
            |c: MemorySystem16| Flat::Done(vec![Computational::MemorySystem(c).into()]),
            |c: PC| Flat::Continue(c.expand()),
            |c: CPU| Flat::Continue(c.expand()),
            |c: Computer| Flat::Continue(c.expand()),
        ],
    )
}

#[cfg(test)]
mod test {
    use std::collections::HashMap;

    use crate::computer::{
        ALU, CPU, Combinational8T, Computer, EightComponentT, PC, flatten, flatten_for_simulation,
        flatten_to_nands,
    };
    use assignments::tests::test_05;
    use simulator::component::{Combinational, count_combinational, count_computational};
    use simulator::nat::N16;
    use simulator::simulate::{MemoryMap, simulate, synthesize};
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
        print!(
            "{}",
            print_graph(&chip.expand::<Combinational8T, _, _, _, _, _, _, _, _, _>())
        );

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

    #[test]
    fn pc_behavior() {
        let chip = PC::chip();

        // When it breaks, it's nice to see what it tried to do
        println!(
            "{}",
            print_graph(&chip.expand::<EightComponentT, _, _, _, _, _, _, _, _, _>())
        );

        let chip = flatten_for_simulation(chip);

        let no_ram = MemoryMap::empty();
        let mut state = simulate::<_, N16, N16>(&chip, no_ram);

        let tick = |state: &mut simulator::simulate::ChipState<N16, N16>| {
            state.set("top_half", true.into());
            state.set("bottom_half", false.into());
            state.ticktock();
        };
        let tock = |state: &mut simulator::simulate::ChipState<N16, N16>| {
            state.set("top_half", false.into());
            state.set("bottom_half", true.into());
            state.ticktock();
        };
        let crank = |mut state: &mut simulator::simulate::ChipState<N16, N16>| {
            tick(&mut state);
            tock(&mut state);
        };

        assert_eq!(state.get("out"), 0u16.into());

        crank(&mut state);

        assert_eq!(state.get("out"), 0u16.into()); // No change: no flags set

        // "Normal" operation: inc is set and the value marches forward:

        state.set("inc", true.into());

        assert_eq!(state.get("out"), 0u16.into()); // No change: previous value still latched

        tick(&mut state);
        assert_eq!(state.get("out"), 0u16.into()); // No change after the "top half" cycle

        tock(&mut state);
        assert_eq!(state.get("out"), 1u16.into()); // Now the incremented address is available

        crank(&mut state);
        assert_eq!(state.get("out"), 2u16.into());

        // Now hold the updated value:

        state.set("inc", false.into());

        crank(&mut state);

        assert_eq!(state.get("out"), 2u16.into());

        // Re-assert inc, but override it with a load:

        state.set("inc", true.into());
        state.set("addr", 0x1234u16.into());
        state.set("load", true.into());

        crank(&mut state);
        assert_eq!(state.get("out"), 0x1234u16.into());

        crank(&mut state);
        assert_eq!(state.get("out"), 0x1234u16.into()); // Load still in effect

        state.set("load", false.into());
        crank(&mut state);
        assert_eq!(state.get("out"), 0x1235u16.into()); // addr ignored now, back to inc

        // Pull the ejection switch:

        state.set("load", true.into()); // Will be ignored while reset is asserted
        state.set("reset", true.into());

        crank(&mut state);
        assert_eq!(state.get("out"), 0u16.into());

        // Specifically test crossing an 8-bit address boundary:
        state.set("reset", false.into());
        state.set("addr", 0x00ffu16.into());
        state.set("load", true.into());
        crank(&mut state);
        assert_eq!(state.get("out"), 0x00ffu16.into());

        state.set("load", false.into());
        crank(&mut state);
        assert_eq!(state.get("out"), 0x0100u16.into()); // low-byte = 0; high-byte = 1
    }

    #[test]
    fn pc_optimal() {
        let chip = flatten(PC::chip());
        // Note: flattening to computational for simplicity, even though only register is needed
        let counts = count_computational(&chip.components);
        assert_eq!(counts.nands, 274); // Compare to 272 (project_03 PC)
        assert_eq!(counts.dffs, 3 * 8); // 2 8-bit registers and a latch; compare to 1x16
    }

    #[test]
    fn cpu_optimal() {
        let chip = flatten(CPU::chip());
        let counts = count_computational(&chip.components);
        assert_eq!(counts.nands, 995); // Compare to 1273
        assert_eq!(counts.dffs, 67); // 6 8-bit registers, 2 8-bit latches (ALU and PC), and 3 1-bit latches for carries and the zr condition; compare to 48 (3x16)
    }

    #[test]
    fn computer_max_behavior() {
        use assignments::project_05::{find_rom, memory_system};
        use assignments::tests::test_05::max_program;

        let chip = flatten(Computer::chip());
        let state = simulate::<_, N16, N16>(&chip, memory_system());

        let pgm = max_program();
        let max_cycles = pgm.len() as u64 * 2;
        find_rom(&state).flash(pgm);

        test_05::test_computer_max_behavior(state, max_cycles);
    }

    #[test]
    fn computer_optimal() {
        let chip = flatten(Computer::chip());
        let counts = count_computational(&chip.components);
        assert_eq!(counts.nands, 995); // Compare to 1273
        assert_eq!(counts.dffs, 67);
        assert_eq!(counts.roms, 1);
        assert_eq!(counts.memory_systems, 1);
    }

    #[test]
    fn computer_wiring() {
        let chip = flatten_for_simulation(Computer::chip());

        let wiring = synthesize(&chip, MemoryMap::empty());

        println!("{wiring}");

        let ops = wiring.op_counts();
        assert_eq!(
            ops.nands + ops.ands,
            29,
            "Really, any reasonable number; just not hundreds"
        );
        assert_eq!(
            ops.shifts, 8,
            "2 half-words get moved in and out of address and data buses 4 times, empirically"
        );
        assert_eq!(
            ops.muxes, 16,
            "Enough to feel like we're skipping some work"
        );
    }
}
