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

Don't try to implement the more academic aspects of the orignal materials. For example, a RAM
implemented out of raw gates is a mere exercise; here we always assume a better-performing,
"native" RAM component.

The majority of the details are being worked out using Claude Code. The meta-goal is to see how
well it's able to handle a project of this complexity, and how much fun it is to do a project like
this that way. See the commit comments for a record of when Claude was helpful and when not.


## Quick Start

Have a rust toolchain...

`cargo run cargo run --release -p computer -- examples/Pong.asm`

An initial, naive simulator ran at about 900Hz (Apple M2, ca. 2026.)

After pre-computing storage location to avoid lookups in the simulation loop, we're up to about 5KHz.


## Simulation

A single chip/computer simulator is implemented. The user/learner writes a description of their
chip design, using the provided Rust API. When the program is compiled, that description is
transformed into a form that can be efficiently simulated. During simulation, the state of
every simulated logic gate is tracked, cycle-by-cycle. *Any* circuit can be simulated, based on
the idealized behavior of clocked and unclocked NAND gates. That is, there's no special provision
for particular computation units, bit widths, or design strategies. On the other hand, there's no
way to exploit non-ideal behavior like propagation delay, etc.

## Support Chips

For the sake of efficiency, a few components are provided to the simulation as primitives; they
are emulated directly in Rust, so their function is fixed. They interface with user circuits in the
same way as any other component.

- Register
    - `bits`: arbitrary word size
    - inputs: `load`, `data[bits]`, `out[bits]`
- (TODO) RAM
    - `data_bits`: arbitrary word size (up to the host word size in bits, most likely 64)
    - `address_bits`: arbitrary address size (limited by available host memory)
    - (TODO) configurable read/write delay, in terms of cycles (default 1)
    - inputs: `address[address_bits]`, `write`, `in[data_bits]`
    - outputs: `out[data_bits]`
- (TODO) I/O
    - `Keyboard`: for reading one word at a time from the keyboard, serial interface, or other simulated device.
    - `TTY`: for writing one word at a time to a printer, screen, serial interface, or other imaginary interface.

## I/O

Terminal-style: see "I/O" above. Characters can be written and read to and from the outside world
using the builtin components for minimal overhead. During the simulation, the components can be
wired to stdin/out, captured for testing, etc.

Graphical displays: character and/or pixel-mode graphics are provided by mapping (a portion of)
a RAM as a screen buffer. The simulator takes care of rendering that data to the actual screen.

## The Chip DSL

The first interface to the simulator is an API for constructing descriptions of circuits. This
description, called a `??? (TBD)`, consists of primitive/native and composite *components*,
whose outputs and inputs are wired together.

Each type of component has a predefined set of inputs and outputs. For example, the primitive
`Nand` has two one-bit inputs, `a` and `b`, and a single one-bit output `out`. An component can
have more interesting inputs and outputs than that, any many user-defined components will.

Components are constructed and used in several separate phases:

### Definition

Any novel components are defined as `struct`s with corresponding `Component` impls.

`fn expand()` specifies how the component behaves, in terms of more primitive components.

For example, a simple circuit might consist of just two Nands, with the output of one connected to
the inputs of the other.

### Construction

The final chip is assembled out of the necessary components. Any component can be realized as a complete
chip; for example, when testing a single logic unit. See `Chip::chip()`.

### Expansion

The complete chip is expanded recursively so that all sub-components are reduced to primitive gates
(except for external components.)

Before and/or after expansion, the graph may be transformed, for example to eliminate unused elements.

### Evaluation

Simple "combinational" circuits, which expand to nothing bu Nand gates, can be evaluated in
a stateless way using `simulator::eval::eval()`.

### Synthesis

The final chip's configuration is converted to a form that can be handled efficiently by
the simulator, using `simulator::simulate::synthesize()`.

### Simulation

The state of entire chip is traced, cycle-by-cycle, simulating the behavior of the chip in full
detail. The internal representation is optimized for speed of simulation, but the simulator might
provide affordances for inspecting internal state for debugging purposes.

"External" components like the keyboard and display are handled by native code in the simulator.
Depending on what components are needed, the simulator can map I/O to the terminal or capture it
for tests, etc.
