/// Alternate Hack CPU implementation, using only 8-bit registers and adders.
///
/// This design uses ~20% fewer gates, but requires 2 cycles to execute each Hack instruction.
/// Mostly, it's a test case for simulating an alternative architecure; this one shares no
/// components with the standard CPU beyond the primitives and single-bit logic.
use assignments::project_01::{And, Mux, Nand, Not, Or};
use assignments::project_02::FullAdder;
use assignments::project_02::{self, HalfAdder};
use assignments::project_05::Decode;
use simulator::component::native;
use simulator::component::{Buffer, Computational};
use simulator::component::{Combinational, Computational16, MemorySystem16, ROM16};
use simulator::nat::N16;
use simulator::{
    Chip, Component, IC, Input1, Input16, Interface, Output, Output16, Reflect, declare::BusRef,
    expand, fixed,
};

use crate::component::{
    EightDecode, EightMemSys, EightROM, Input8, Latch1, Latch8, Output8, Register8,
};

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

/// out = a + carry_in (8-bit, with carry out)
#[derive(Clone, Reflect, Chip)]
pub struct Inc8 {
    pub a: Input8,
    pub carry_in: Input1,
    pub out: Output8,
    pub carry_out: Output,
}
impl Component for Inc8 {
    type Target = project_02::Project02Component;

    expand! { |this| {
        _carry_out: (0..8).fold(this.carry_in, |carry, i| {
            add: HalfAdder {
                a: this.a.bit(i),
                b: carry,
                sum: this.out.bit(i),
                carry: if i == 7 { this.carry_out } else { Output::new() },
            },
            add.carry.into()
        }),
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

/// Slice a 16-bit bus into high and low half-words.
#[derive(Clone, Reflect, Chip)]
pub struct Split {
    pub a: Input16,

    pub hi: Output8,
    pub lo: Output8,
}

impl Component for Split {
    type Target = Combinational8;

    expand! { |this| {
        for i in 0..8 {
            _lo: Buffer { a: this.a.bit(i), out: this.lo.bit(i) },
            _hi: Buffer { a: this.a.bit(8+i), out: this.hi.bit(i) },
        }
    }}
}

/// Assemble high and low half-words into a 16-bit signal.
#[derive(Clone, Reflect, Chip)]
pub struct Join {
    pub hi: Input8,
    pub lo: Input8,

    pub out: Output16,
}

impl Component for Join {
    type Target = Combinational8;

    expand! { |this| {
        for i in 0..8 {
            _lo: Buffer { a: this.lo.bit(i), out: this.out.bit(i) },
            _hi: Buffer { a: this.hi.bit(i), out: this.out.bit(8+i) },
        }
    }}
}

#[derive(Clone, Reflect, Component)]
pub enum Combinational8 {
    #[delegate]
    Project02(project_02::Project02Component),
    Decode(EightDecode),
    Mux8(Mux8),
    Not8(Not8),
    And8(And8),
    Inc8(Inc8),
    Add8(Add8),
    Nand8Way(Nand8Way),
    Zero8(Zero8),
    Neg8(Neg8),
    ALU(ALU),
    Split(Split),
    Join(Join),
}

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

impl Component for Latch8 {
    type Target = EightComponent;

    expand! { |this| {
        reg: Register8 { data_in: this.data_in, write: fixed(1), data_out: this.data_out },
    }}
}

impl Component for EightDecode {
    type Target = Combinational8;
    fn expand(&self) -> Option<IC<Combinational8>> {
        self.0.expand().map(|ic| IC {
            name: ic.name,
            intf: ic.intf,
            components: ic.components.into_iter().map(Into::into).collect(),
        })
    }
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

impl Component for PC {
    type Target = EightComponent;

    expand! { |this| {
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

        lo: Register8 { data_in: next2_lo.out.into(), write: this.bottom_half, data_out: lo_out },
        hi: Register8 { data_in: next2_hi.out.into(), write: this.bottom_half, data_out: hi_out },

        // Latch Inc result for next cycle.
        latch: Latch8 { data_in: inc.out.into(), data_out: latch_out },
    }}
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

impl Component for CPU {
    type Target = EightComponent;

    expand! { |this| {
        top_half: forward Output::new(),
        bottom_half: forward Output::new(),

        alu_latch_out: forward Output8::new(),
        zr_latch_out: forward Output::new(),
        carry_latch_out: forward Output::new(),

        reg_a_lo_out: forward Output8::new(),
        reg_a_hi_out: forward Output8::new(),

        reg_d_lo_out: forward Output8::new(),
        reg_d_hi_out: forward Output8::new(),

        decode: EightDecode(Decode {
            instr: this.instr,

            is_c: Output::new(),
            is_a: Output::new(),

            read_m: Output::new(),

            zx: Output::new(), nx: Output::new(),
            zy: Output::new(), ny: Output::new(),
            f:  Output::new(), no: Output::new(),

            write_a: Output::new(), write_m: Output::new(), write_d: Output::new(),

            jmp_lt:  Output::new(), jmp_eq:  Output::new(), jmp_gt:  Output::new(),
        }),

        x_src: Mux8 { a0: reg_d_lo_out.into(), a1: reg_d_hi_out.into(), sel: bottom_half.into(), out: Output8::new() },

        // === load_a = is_a OR write_a ===
        load_a: Or { a: decode.is_a.into(), b: decode.write_a.into(), out: Output::new() },

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

        alu: ALU {
            x:   x_src.out.into(),
            y:   y_src.out.into(),
            carry_in: carry_latch_out.into(),
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
        is_pos:   And { a: not_ng.out.into(), b: not_zr.out.into(), out: Output::new() },
        // Jump signals already gated with is_c in Decode.
        jlt_and:  And { a: decode.jmp_lt.into(), b: alu.ng.into(), out: Output::new() },
        jeq_and:  And { a: decode.jmp_eq.into(), b: alu.zr.into(), out: Output::new() },
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

        alu_latch: Latch8 { data_in: alu.out.into(), data_out: alu_latch_out },
        zr_latch: Latch1 { data_in: alu.zr.into(), data_out: zr_latch_out },
        carry_latch: Latch1 { data_in: alu.carry_out.into(), data_out: carry_latch_out },

        reg_a_lo: Register8 { data_in: a_data_lo.out.into(), write: load_a.out.into(), data_out: reg_a_lo_out },
        reg_a_hi: Register8 { data_in: a_data_hi.out.into(), write: load_a.out.into(), data_out: reg_a_hi_out },

        reg_d_lo: Register8 { data_in: alu_latch_out.into(), write: decode.write_d.into(), data_out: reg_d_lo_out },
        reg_d_hi: Register8 { data_in: alu.out.into(),       write: decode.write_d.into(), data_out: reg_d_hi_out },

        next_cycle: Not { a: top_half.into(), out: bottom_half },
        cycle_dff: Latch1 { data_in: next_cycle.out.into(), data_out: top_half },
    }}
}

#[derive(Clone, Reflect, Chip)]
pub struct Computer {
    /// A way to force the CPU to return to a known state (i.e. jump to address 0)
    pub reset: Input1,

    /// Useful for debugging, but also acts as a root for traversing the graph
    pub pc: Output16,
}

impl Component for Computer {
    type Target = EightComponent;

    expand! { |this| {
        mem_out: forward Output16::new(),

        rom: EightROM(ROM16 {
            size: 32 * 1024,
            addr: this.pc.into(),
            out:  Output16::new(),
        }),

        cpu: CPU {
            reset:        this.reset,
            pc:           this.pc,
            instr:        rom.out.into(),
            mem_data_out: Output16::new(),
            mem_write:    Output::new(),
            mem_addr:     Output16::new(),
            mem_data_in:  mem_out.into(),
        },

        memory: EightMemSys(MemorySystem16 {
            addr:     cpu.mem_addr.into(),
            write:    cpu.mem_write.into(),
            data_in:  cpu.mem_data_out.into(),
            data_out: mem_out,
        }),
    }}
}

#[derive(Reflect, Component)]
pub enum EightComponent {
    #[delegate]
    Combinational8(Combinational8),
    #[primitive]
    Register(Register8),
    #[primitive]
    Latch1(Latch1),
    #[primitive]
    ROM(EightROM),
    #[primitive]
    MemorySystem(EightMemSys),
    Latch8(Latch8),
    PC(PC),
    CPU(CPU),
    Computer(Computer),
}

impl From<EightROM> for Computational16 {
    fn from(r: EightROM) -> Self {
        Computational::ROM(r.0)
    }
}

impl From<EightMemSys> for Computational16 {
    fn from(m: EightMemSys) -> Self {
        Computational::MemorySystem(m.0)
    }
}

/// Recursively expand until only Nands, Registers, RAMs, ROMs, and MemorySystems are left.
pub fn flatten<C: Reflect + Into<EightComponent>>(chip: C) -> IC<Computational16> {
    fn go(comp: EightComponent) -> Vec<Computational16> {
        match comp {
            EightComponent::Combinational8(c) => match c.expand() {
                Some(ic) => ic
                    .components
                    .into_iter()
                    .flat_map(|c| go(c.into()))
                    .collect(),
                None => project_02::flatten(match c {
                    Combinational8::Project02(p) => p,
                    other => panic!("Did not reduce to primitive: {:?}", other.name()),
                })
                .components
                .into_iter()
                .map(Into::into)
                .collect(),
            },
            EightComponent::Register(r) => vec![Computational::Register(r.into())],
            EightComponent::Latch1(l) => vec![Computational::Register(l.into())],
            EightComponent::ROM(r) => vec![Computational::ROM(r.0)],
            EightComponent::MemorySystem(m) => vec![Computational::MemorySystem(m.0)],
            other => match other.expand() {
                Some(ic) => ic.components.into_iter().flat_map(go).collect(),
                None => panic!("Did not reduce to primitive: {:?}", other.name()),
            },
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
        match comp {
            EightComponent::Combinational8(Combinational8::Project02(p)) => {
                project_02::flatten_for_simulation(p).components
            }
            EightComponent::Combinational8(Combinational8::Mux8(c)) => {
                vec![
                    native::Mux {
                        a0: c.a0,
                        a1: c.a1,
                        sel: c.sel,
                        out: c.out,
                    }
                    .into(),
                ]
            }
            EightComponent::Combinational8(c) => match c.expand() {
                Some(ic) => ic
                    .components
                    .into_iter()
                    .flat_map(|c| go(c.into()))
                    .collect(),
                None => panic!("Did not reduce to primitive: {:?}", c.name()),
            },
            EightComponent::Register(r) => vec![Computational::Register(r.into()).into()],
            EightComponent::Latch1(l) => vec![Computational::Register(l.into()).into()],
            EightComponent::ROM(r) => vec![Computational::ROM(r.0).into()],
            EightComponent::MemorySystem(m) => vec![Computational::MemorySystem(m.0).into()],
            other => match other.expand() {
                Some(ic) => ic.components.into_iter().flat_map(go).collect(),
                None => panic!("Did not reduce to primitive: {:?}", other.name()),
            },
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

    use crate::computer::{ALU, CPU, Computer, PC, flatten, flatten_to_nands};
    use assignments::tests::test_05;
    use simulator::component::{Combinational, count_combinational, count_computational};
    use simulator::nat::N16;
    use simulator::simulate::{MemoryMap, simulate};
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

    #[test]
    fn pc_behavior() {
        let chip = PC::chip();

        // When it breaks, it's nice to see what it tried to do
        print!("{}", print_graph(&chip));

        let chip = flatten(chip);

        let no_ram = MemoryMap::new(vec![]);
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
        assert_eq!(counts.nands, 221); // Compare to 223
        assert_eq!(counts.registers, 3); // 3x8 bits; compare to 1x16
    }

    #[test]
    fn cpu_optimal() {
        let chip = flatten(CPU::chip());
        let counts = count_computational(&chip.components);
        assert_eq!(counts.nands, 776); // Compare to 1126
        assert_eq!(counts.registers, 11); // 6 8-bit registers, 2 8-bit latches (ALU and PC), and a couple of 1-bit latches for carries and the zr condition
    }

    #[test]
    fn computer_max_behavior() {
        use assignments::project_05::{find_rom, memory_system};
        use assignments::tests::test_05::max_program;

        let chip = flatten(Computer::chip());
        let state = simulate::<_, N16, N16>(&chip, memory_system());

        find_rom(&state).flash(max_program());

        test_05::test_computer_max_behavior(state, 100);
    }

    #[test]
    fn computer_optimal() {
        let chip = flatten(Computer::chip());
        let counts = count_computational(&chip.components);
        assert_eq!(counts.nands, 776); // Compare to 1126
        assert_eq!(counts.registers, 11);
        assert_eq!(counts.roms, 1);
        assert_eq!(counts.memory_systems, 1);
    }
}
