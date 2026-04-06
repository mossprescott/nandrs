# Alternate CPUs

A playground for doing the same with less (or more.)


## Double

Dispatches more instructions per cycle by observing when:
- the *current* instruction is not a (taken) branch, and
- the *following* instruction is a simple load to A ("@...")

In that case, all the usual handling occurs, and then:
- the new value for A is written to the register and latched into the RAM
- the PC is incremented by 2

That's conceptually simple, but it involves considerable hardware resources:
- an extra register to latch the address of the second instruction
- an entire duplicate *ROM*, since the current design only has a single addr/data port
- a couple of extra Inc16 variants, to construct instruction addresses. Currently this is doubling the total number of FullAdder primitives involved
- a handful of extra logic gates to orchestrate

In simulation, the clock speed is heavily impacted, probably by the extra adders, but even so the
extra work getting done on each cycle brings the frame rate to roughly parity.


## Eight

Conversely, get half as much done per cycle by using only an 8-bit ALU and address incrementer.




## Results

Running examples/Pong.asm:

|                                                | gates       | init         | speed           |
|------------------------------------------------|-------------|--------------|-----------------|
| [project_05](../assignments/src/project_05.rs) | 1273        | 3.9m         | 2.0 MHz, 30 fps |
| [double](double/src/computer.rs)               | 1581 (+24%) | 2.8m  (-28%) | 1.4 MHz, 30 fps |
| [eight](eight/src/computer.rs)                 |  995 (-22%) | 7.8m (+100%) | 1.3 MHz, 20 fps |

- *gates*: number of Nands, including flattened adders and muxes, but not registers and the whole memory system.
- *init*: number of cycles to reach the "main.main" label
- *speed*: as shown in the UI
