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

## Results

|                                                | "gates"  | init(1)  | "speed"       |
|------------------------------------------------|------|--------------|---------------|
| [project_05](../assignments/src/project_05.rs) | ...  | 3.9m         | 1.3MHz, 20fps |
| [double](double/src/computer.rs)               | (+?) | 2.8m  (-28%) |  800Hz, 20fps |