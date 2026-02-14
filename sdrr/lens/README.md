# One ROM Lens

This directory contains the source code for the One ROM Lens, a tool which runs the One ROM PIO algorithms in a browser and visualizes the results.

To build, first build the firmwre.  This must be done using the original `make` approach, as opposed to the newer `scripts/onerom.sh` approach.

```bash 
HW_REV=fire-24-e MCU=rp2350 ROM_CONFIGS=file=images/test/0_63_8192.rom,type=2364,cs1=0 make
```

Then, build and run One ROM Lens:

```bash
make -C sdrr -f lens.mk run
```

Point a browser at `http://localhost:8000` to access the One ROM Lens interface.
