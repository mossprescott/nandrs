# Nand to Tetris: Rust Edition

This is yet another implementation of the gate-level chip simulation tools originally provided
with the "From Nand to Tetris" course.

This time in Rust, with Claude's help, and with a little less focus on completeness and
documentation.

## Goals

Implement the same simulated hardware platform as Nand-to-Tetris, this time in the Rust toolchain.

Provide a point of comparison with the implementation at https://github.com/mossprescott/pynand.

Support alternative chip designs equally well:
- different bit widths, RAM/ROM sizes, etc.
- no special-casing chips in the simulator (i.e. Add16 isn't special)
- simulate every gate and wire, with acceptable performance
- that is, run any reasonable design at approx. 1 MHz or better.

On the other hand, some concessions are made in the interest of simulation speed, as long as they don't
get in the way of experimenting:
- max bus width is 64 bits; you could build a bigger chip with extra hassle, but you won't
- a multi-bit Mux component is provided as primitive; this allows the simulator to easily identify
  portions of the chip that are active in each cycle

Focus on *processor* design considerations. Overall system design issues like bus timing, memory
hierarchies, etc. are beside the point. I want to play with different ISAs, functional unit
selection and design, things like that.

Don't try to implement the more academic aspects of the orignal materials. For example, a RAM
implemented out of raw gates is a mere exercise; here we always assume a better-performing,
"native" RAM component.

The majority of the details are being worked out using Claude Code. The meta-goal is to see how
well it's able to handle a project of this complexity, and how much fun it is to do a project like
this that way. See the commit comments for a record of when Claude was helpful and when not.


## Quick Start

Have a rust toolchain...

`cargo run --release -p computer -- examples/Pong.asm --2x`


## Performance/Results

An initial, naive simulator ran at about 900Hz (Apple M2, ca. 2026.)

After pre-computing storage location to avoid lookups in the simulation loop, we're up to about
5KHz.

Storing all the state in Vec<u64> with dense indices instead of HashMaps: about 100KHz.

Making `Mux` for arbitrary width a primitive: 175KHz. Still evaluating all the logic for both inputs, so far.

Conditional evaluation of just the immediate inputs of each Mux branch; all the bits of just the
active branch: 250KHz now.

Undo earlier optimization so we have more muxes and less total gates; a little more gating (inputs to Add16 this time): 440KHz.

Collapsed Nand/Not to unitary AndWiring op: 550KHz.

Pruned unused outputs, including the carry-out bit from Add16, which allowed the mux folding pass to pull in the whole adder, or something like that: >900KHz.

Made FullAdder (the bit-slice adder) primitive: 1.25MHz. Actually surprisingly little speed-up
considering FullAdder was nine gates/operations.

Things to look at next:
- more than one place where there are 16 parallel Nands (i.e. And16 or Not16, probably); peephole optimize them?

## Simulation

A single chip/computer [simulator](simulator/src/simulate/mod.rs) is implemented. The user/learner
writes a description of their chip design, using the provided Rust API. When the program is
compiled, that description is transformed into a form that can be efficiently simulated. During
simulation, the state of every simulated logic gate is tracked, cycle-by-cycle. *Any* circuit can be
simulated, based on the idealized behavior of clocked and unclocked NAND gates. That is, there's no
special provision for particular computation units, bit widths, or design strategies. On the other
hand, there's no way to exploit non-ideal behavior like propagation delay, etc.

### Combinational Evaluation

For purely combinational chips (no registers or memory), a simple, tree-walking
[evaluator](simulator/src/eval.rs) is also provided. This evaluator isn't intended to be fast; it
was easy to write, probably doesn't have bugs, and it can be a helpful sanity check when hacking on
the fancier simulator.


## Primitives

In addition to the essential primitive, `Nand`, the following are provided to help define designs in
a natural way that can also be simulated efficiently:

- `Const`: no inputs, output is a fixed set of bits. No runtime cost.
- `Buffer`: passes its singl, singe-bit input directly to its output. This is just a convenience
  components can use, often to connect inputs directly to outputs. No runtime cost.
- `Mux`: two (muti-bit) inputs, and a `sel` input controlling which one is used. During simulation, `sel` is evaluated first; then the simulator only evaluates as needed for the "active" input.


## Support Chips

For the sake of efficiency, a few components are provided to the simulation as primitives; they
are emulated directly in Rust, so their function is fixed. They interface with user circuits in the
same way as any other component.

- Register
    - `bits`: arbitrary word size
    - inputs: `load`, `data[bits]`, `out[bits]`
- ROM
    - read-only memory, configured via `flash()`
    - `data_bits`: arbitrary word size (up to the host word size in bits, most likely 64)
    - `address_bits`: arbitrary address size (limited by available host memory)
    - inputs: `addr[address_bits]`,
    - outputs: `out[data_bits]`
    - Note: `addr` can be applied and `out` read within the same cycle.
- RAM
    - `data_bits`: arbitrary word size (up to the host word size in bits, most likely 64)
    - `address_bits`: arbitrary address size (limited by available host memory)
    - inputs: `addr[address_bits]`, `write`, `data_in[data_bits]`
    - outputs: `data_out[data_bits]`
    - `addr` is latched; the address that was applied in the *previous* cycle is   in effect
    - TODO: configurable "read" latency beyond the one cycle that the Hack design requires.
- (TODO) I/O
    - `Keyboard`: for reading one word at a time from the keyboard, serial interface, or other simulated device.
    - `TTY`: for writing one word at a time to a printer, screen, serial interface, or other imaginary interface.
- MemoryMap
    - exposes the same interface as RAM, and internally maps writes and reads to one or more
      components (RAMs, ROMs, etc.), so they all share a flat address space.

## I/O

Terminal-style: see "I/O" above. Characters can be written and read to and from the outside world
using the builtin components for minimal overhead. During the simulation, the components can be
wired to stdin/out, captured for testing, etc.

Graphical displays: character and/or pixel-mode graphics are provided by mapping (a portion of) a
RAM as a screen buffer. The [harness](computer/src/main.rs) takes care of rendering that data to the
actual screen.

## The Chip DSL

The first interface to the simulator is an API for constructing descriptions of circuits. This
description, a `struct` which implements the `Component` trait, consists of primitive/native and
composite *components*, whose outputs and inputs are wired together.

Each type of component has a predefined set of inputs and outputs. For example, the primitive
`Nand` has two one-bit inputs, `a` and `b`, and a single one-bit output `out`. An component can
have more interesting inputs and outputs than that, d many user-defined components will.

Components are constructed and used in several separate phases:

### Definition

Any novel components are defined as `struct`s with corresponding `Component` impls.

`fn expand()` specifies how the component behaves, in terms of more primitive components.

For example, a simple circuit might consist of just two `Nand`s, with the output of one connected to
the inputs of the other.

### Construction

The final chip is assembled out of the necessary components. Any component can be realized as a
complete chip; for example, when testing a single logic unit. See `Chip::chip()`, which is typically
derived.

### Expansion

The complete chip is expanded recursively (flattened) so that all sub-components are reduced to just
the pre-defined primitives and support chips descibed above.

Before and/or after expansion, the graph may be transformed, for example to eliminate unused
elements.

### Evaluation

Simple "combinational" circuits, which expand to nothing but Nand gates, can be evaluated in
a stateless way using `simulator::eval::eval()`.

### Synthesis

The final chip's configuration is converted to a form that can be handled efficiently by
the simulator, using `simulator::simulate::synthesize()`.

### Simulation

The state of entire chip is traced, cycle-by-cycle, simulating the behavior of the chip in full
detail. The internal representation is optimized for speed of simulation, but the simulator might
provide affordances for inspecting internal state for debugging purposes.

"External" components like the keyboard and display are handled by native code in the harness.
Depending on what components are needed, the harness can map I/O to the terminal or capture it
for tests, etc.
