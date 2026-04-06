use crate::project_01::{And, And16, Buffer, Mux, Mux16, Nand, Not, Not16};
use crate::project_02::{ALU, Add16, FullAdder, HalfAdder, Inc16, Nand16Way, Neg16, Zero16};
use frunk::coproduct::CoprodInjector;
use frunk::{Coprod, hlist};
use simulator::component::{DFF, Sequential};
use simulator::declare::{BusRef, Interface};
use simulator::{
    self, Chip, Flat, IC, Input1, Input16, Output16, Reflect, expand, fixed, flatten_g,
};

pub type Project03 = Coprod!(
    Nand, Buffer, Not, And, Mux, Mux16, Not16, And16, HalfAdder, FullAdder, Inc16, Add16,
    Nand16Way, Zero16, Neg16, ALU, DFF, Register16, PC
);

/// Recursively expand until only Nands and Registers are left.
pub fn flatten<C, Idx>(chip: C) -> IC<Sequential>
where
    C: Reflect,
    Project03: CoprodInjector<C, Idx>,
{
    flatten_g::<C, Project03, Idx, Sequential, _>(
        chip,
        "flat",
        hlist![
            |c: Nand| Flat::Done(vec![Sequential::Nand(c)]),
            |c: Buffer| Flat::Done(vec![Sequential::Buffer(c)]),
            |c: Not| Flat::Continue(c.expand()),
            |c: And| Flat::Continue(c.expand()),
            |c: Mux| Flat::Continue(c.expand()),
            |c: Mux16| Flat::Continue(c.expand()),
            |c: Not16| Flat::Continue(c.expand()),
            |c: And16| Flat::Continue(c.expand()),
            |c: HalfAdder| Flat::Continue(c.expand()),
            |c: FullAdder| Flat::Continue(c.expand()),
            |c: Inc16| Flat::Continue(c.expand()),
            |c: Add16| Flat::Continue(c.expand()),
            |c: Nand16Way| Flat::Continue(c.expand()),
            |c: Zero16| Flat::Continue(c.expand()),
            |c: Neg16| Flat::Continue(c.expand()),
            |c: ALU| Flat::Continue(c.expand()),
            |c: DFF| Flat::Done(vec![Sequential::DFF(c)]),
            |c: Register16| Flat::Continue(c.expand()),
            |c: PC| Flat::Continue(c.expand()),
        ],
    )
}

/// 16-bit wide register made out of DFFs.
#[derive(Clone, Reflect, Chip)]
pub struct Register16 {
    pub data_in: Input16,
    pub write: Input1,
    pub data_out: Output16,
}

impl Register16 {
    expand!([Mux16, DFF], |this| {
        next: Mux16 { a0: this.data_out.into(), a1: this.data_in, sel: this.write, out: Output16::new() },
        for i in 0..16 {
            // next: Mux { a0: this.data_out.bit(i).into(), a1: this.data_in.bit(i).into(), sel: this.write, out: Output::new() },
            dff: DFF { a: next.out.bit(i).into(), out: this.data_out.bit(i) },
        }
    });
}

/// Program counter component, including a register storing the current instruction address.
///
/// When more than one flag is set, "reset" supercedes "load", which supercedes "inc".
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

impl PC {
    expand!([Inc16, Mux16, Register16], |this| {
        // Note: no special ceremony needed for back-references to the register's output, because
        // that wire is already declared as the output "out".
        inc:   Inc16  { a: this.out.into(), out: Output16::new() },
        inced: Mux16  { a0: this.out.into(),   a1: inc.out.into(), sel: this.inc,   out: Output16::new() },
        loaded: Mux16 { a0: inced.out.into(),  a1: this.addr,      sel: this.load,  out: Output16::new() },
        reset: Mux16  { a0: loaded.out.into(), a1: fixed(0),       sel: this.reset, out: Output16::new() },
        reg:   Register16 {
            data_in:  reset.out.into(),
            write:    fixed(1),
            data_out: this.out,
        },
    });
}
