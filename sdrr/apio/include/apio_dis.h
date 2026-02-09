// Copyright (C) 2026 Piers Finlayson <piers@piers.rocks>
//
// MIT License

// apio - Inline PIO assembler for RP2350
//
// Disassembly routines

#include <stdint.h>

void apio_instruction_decoder(uint32_t instr, char out_str[64], uint8_t start_offset);
void apio_log_sm(
    const char *sm_name,
    uint8_t pio_block,
    uint8_t pio_sm,
    uint16_t *instr_scratch,
    uint8_t first_instr,
    uint8_t start,
    uint8_t end
);

