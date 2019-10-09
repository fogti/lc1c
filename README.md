# lc1c

My own high-level LC1 asm compiler

LC1 resources:
https://www.tu-chemnitz.de/informatik/friz/Grundl-Inf/Rechnerarchitektur/LC1/

## rust implementation features

The Rust implementation currently lacks many features of the original C++ implementation.

### Dropped
* relative address mode (droppped because too complex to get right, maybe later)

### Planned
* [ ] label resolving
* [ ] indirect consts address mode
* [X] flat optimization (a.k.a. peephole optimizations)
* [ ] deep optimization (a.k.a. control-flow analysis, dead-code detection, ...)
