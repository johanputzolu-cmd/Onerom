#!/usr/bin/env bash

# Script to build One ROM firmware and flash it to device using a JSON chip
# config file.

set -e

HW_REV=fire-24-e MCU=rp2350 ROM_CONFIGS=file=images/test/0_63_8192.rom,type=2364,cs1=0 make
make -C sdrr -f wasm.mk clean
make -C sdrr -f wasm.mk run
