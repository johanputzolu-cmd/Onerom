# Fire 24 Rev C

**Verified**

23xx Fire (RP2350 24 pin) combined USB+SWD One ROM PCB.  Includes 2 image select jumpers.  Supports PIO and CPU serving algorithms.

## Contents

- [Schematic](./fire-24-c-schematic.pdf)
- [Fab Files](fab/)
- [KiCad Design Files](kicad/)
- [Errata](#errata)
- [Notes](#notes)
- [Changelog](#changelog)

## Errata

This design incorrectly orders the CS pins, so that they are non-contiguous in the 2332 case.  This is OK, as the PIO algorithm has been updated to handle this case.  See issue #76 for details.

## Notes

None

## Changelog

Changes from USB rev B
- Moved SWD pads from underside to dedicated programming pins at the top.
- Reduced image select pins from 4 to 2, to make room for SWD pins.
- Other minor changes.