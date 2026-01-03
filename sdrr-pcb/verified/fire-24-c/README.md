# Fire 24 Rev C

**Univerified** - This is a bugged design.  CS1 and CS2 (A12) are non-contiguous, meaning the 2332 ROM cannot be properly served by the PIO algorithm.

23xx Fire (RP2350 24 pin) combined USB+SWD One ROM PCB.  Includes 2 image select jumpers.  Supports PIO and CPU serving algorithms.

## Contents

- [Schematic](./fire-24-c-schematic.pdf)
- [Fab Files](fab/)
- [KiCad Design Files](kicad/)
- [Errata](#errata)
- [Notes](#notes)
- [Changelog](#changelog)

## Errata

None

## Notes

None

## Changelog

Changes from USB rev B
- Moved SWD pads from underside to dedicated programming pins at the top.
- Reduced image select pins from 4 to 2, to make room for SWD pins.
- Other minor changes.