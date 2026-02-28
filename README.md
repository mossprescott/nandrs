# Nand to Tetris: Rust Edition

This is yet another implementation of the gate-level chip simulation tools originally provided
with the "From Nand to Tetris" course.

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

- (TODO: if needed for performance) Register
    - `bits`: arbitrary word size
    - inputs: `write`, `in[bits]`, `out[bits]`
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
