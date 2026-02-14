// Copyright (C) 2026 Piers Finlayson <piers@piers.rocks>
//
// MIT License

// One ROM WASM API

// Causes declarations in stubbed out code, so One ROM firmware compiles
#define TEST_MAIN_C

#include <stdio.h>
#include <include.h>
#include "roms.h"
#include <test/stub.h>
#include <apio.h>
#include <epio.h>
#include <epio_wasm.h>

static epio_t *g_epio = NULL;
static uint8_t addr_pins[32];
static uint8_t data_pins[16];
static uint8_t cs1_pin, cs2_pin, cs3_pin, x1_pin, x2_pin;

static void setup_pin_mappings(void) {
    // Address pins
    for (int ii = 0; ii < 16; ii++) {
        addr_pins[ii] = sdrr_info.pins->addr[ii];
    }
    for (int ii = 0; ii < 8; ii++) {
        addr_pins[16 + ii] = sdrr_info.pins->addr2[ii];
    }
    
    // Data pins
    for (int ii = 0; ii < 8; ii++) {
        data_pins[ii] = sdrr_info.pins->data[ii];
    }
    for (int ii = 0; ii < 8; ii++) {
        data_pins[8 + ii] = sdrr_info.pins->data2[ii];
    }
    
    // Control pins
    cs1_pin = sdrr_info.pins->cs1;
    cs2_pin = sdrr_info.pins->cs2;
    cs3_pin = sdrr_info.pins->cs3;
    x1_pin = sdrr_info.pins->x1;
    x2_pin = sdrr_info.pins->x2;
}

// Initialize the One ROM emulator
// Returns epio_t* handle on success, NULL on failure.  This handle can be used
// to directly access epio functions if needed.
EPIO_EXPORT epio_t *onerom_init(void) {
    if (g_epio) {
        return 0; // Already initialized
    }
    
    // Set up pin mappings
    setup_pin_mappings();
    
    // Run the firmware to set up the PIO programs
    firmware_main();
    
    // Check PIOs were enabled
    if (_apio_emulated_pio.pios_enabled != 1) {
        return 0;
    }
    
    // Start the PIO emulator
    g_epio = epio_from_apio();
    if (!g_epio) {
        return 0;
    }
    
    // Load ROM data into SRAM
    uint32_t *source = get_ram_rom_image_table_aligned();
    epio_sram_set(g_epio, 0x20000000, (uint8_t *)source, 512*1024);
    
    return g_epio;
}

static const sdrr_rom_info_t *get_first_rom_info(void) {
    if (sdrr_rom_set_count == 0) {
        return NULL; // No ROM sets available
    }

    // Get the first ROM image
    const sdrr_rom_set_t *rom_set_0 = &rom_set[0];
    if (rom_set_0->rom_count == 0) {
        return NULL; // No ROMs in the set
    }

    return rom_set_0->roms[0];
}

EPIO_EXPORT int32_t onerom_lens_get_rom_size(void) {
    const sdrr_rom_info_t *rom = get_first_rom_info();
    if (!rom) {
        return -1; // No ROM available
    }

    assert(rom->rom_type < NUM_CHIP_TYPES);
    uint32_t size = chip_size_from_type[rom->rom_type];

    return size;
}

EPIO_EXPORT uint8_t onerom_lens_get_num_data_bits(void) {
    const sdrr_rom_info_t *rom = get_first_rom_info();
    if (!rom) {
        return 0; // No ROM available
    }

    assert(rom->rom_type < NUM_CHIP_TYPES);
    uint8_t data_bits = 8;
    if (rom->rom_type == CHIP_TYPE_27C400) {
        data_bits = 16;
    }

    return data_bits;
}

// Step the emulator forward by N cycles
EPIO_EXPORT void onerom_step(uint32_t cycles) {
    if (g_epio) {
        epio_step_cycles(g_epio, cycles);
    }
}

// Drive address and control lines
// addr: address to drive (up to 24 bits)
// num_addr_bits: number of address bits to drive (13, 14, 15, etc.)
// cs1, cs2, cs3: chip select states (0=low, 1=high, 2=not driven)
// x1, x2: additional control signals (0=low, 1=high, 2=not driven)
EPIO_EXPORT void onerom_drive_pins(
    uint32_t addr,
    uint8_t num_addr_bits,
    uint8_t cs1,
    uint8_t cs2,
    uint8_t cs3,
    uint8_t x1,
    uint8_t x2
) {
    if (!g_epio) return;
    
    uint64_t drive_mask = 0;
    uint64_t level_mask = 0;
    
    // Address lines
    for (int ii = 0; ii < num_addr_bits; ii++) {
        drive_mask |= (1ULL << addr_pins[ii]);
        if (addr & (1 << ii)) {
            level_mask |= (1ULL << addr_pins[ii]);
        }
    }
    
    // Control lines
    if (cs1 < 2) {
        drive_mask |= (1ULL << cs1_pin);
        if (cs1) level_mask |= (1ULL << cs1_pin);
    }
    if (cs2 < 2) {
        drive_mask |= (1ULL << cs2_pin);
        if (cs2) level_mask |= (1ULL << cs2_pin);
    }
    if (cs3 < 2) {
        drive_mask |= (1ULL << cs3_pin);
        if (cs3) level_mask |= (1ULL << cs3_pin);
    }
    if (x1 < 2) {
        drive_mask |= (1ULL << x1_pin);
        if (x1) level_mask |= (1ULL << x1_pin);
    }
    if (x2 < 2) {
        drive_mask |= (1ULL << x2_pin);
        if (x2) level_mask |= (1ULL << x2_pin);
    }
    
    epio_drive_gpios_ext(g_epio, drive_mask, level_mask);
}

// Stop driving all GPIOs
EPIO_EXPORT void onerom_release_pins(void) {
    if (g_epio) {
        epio_drive_gpios_ext(g_epio, 0, 0);
    }
}

// Read the data pins and return as a byte
EPIO_EXPORT uint32_t onerom_read_data(uint8_t data_bits) {
    if (!g_epio) return 0;
    
    uint64_t gpio_state = epio_read_gpios_ext(g_epio);
    uint32_t data = 0;
    
    for (int ii = 0; ii < data_bits; ii++) {
        if (gpio_state & (1ULL << data_pins[ii])) {
            data |= (1 << ii);
        }
    }
    
    return data;
}

// Read all GPIO states as a 64-bit value
EPIO_EXPORT uint64_t onerom_read_gpios(void) {
    if (!g_epio) return 0;
    return epio_read_gpios_ext(g_epio);
}

// Get the GPIO number for a specific address pin
EPIO_EXPORT uint8_t onerom_get_addr_pin(uint8_t bit) {
    if (bit < 32) {
        return addr_pins[bit];
    }
    return 0xFF;
}

// Get the GPIO number for a specific data pin
EPIO_EXPORT uint8_t onerom_get_data_pin(uint8_t bit) {
    if (bit < 16) {
        return data_pins[bit];
    }
    return 0xFF;
}

EPIO_EXPORT int onerom_get_pio_disassembly(
    char *buffer,
    size_t buffer_size
) {
    int wrote, total_written = 0;

    if (!g_epio) return -1;

    wrote = epio_disassemble_sm(g_epio, 0, 0, buffer, buffer_size);
    if (wrote < 0) {
        return -1;
    }
    buffer += wrote-1;
    buffer_size -= wrote-1;
    total_written += wrote-1;

    // Add 2 \ns between SMs if we have space
    if (buffer_size > 2) {
        *buffer++ = '\n';
        *buffer++ = '\n';
        buffer_size -= 2;
        total_written += 2;
    } else {
        return -1;
    }

    wrote = epio_disassemble_sm(g_epio, 0, 1, buffer, buffer_size);
    if (wrote < 0) {
        return -1;
    }
    buffer += wrote-1;
    buffer_size -= wrote-1;
    total_written += wrote-1;

    // Add 2 \ns between SMs if we have space
    if (buffer_size > 2) {
        *buffer++ = '\n';
        *buffer++ = '\n';
        buffer_size -= 2;
        total_written += 2;
    } else {
        return -1;
    }

    wrote = epio_disassemble_sm(g_epio, 0, 2, buffer, buffer_size);
    if (wrote < 0) {
        return -1;
    }
    buffer += wrote;
    buffer_size -= wrote;
    total_written += wrote;

    return total_written;
}

// Get control pin GPIO numbers
EPIO_EXPORT uint8_t onerom_get_cs1_pin(void) { return cs1_pin; }
EPIO_EXPORT uint8_t onerom_get_cs2_pin(void) { return cs2_pin; }
EPIO_EXPORT uint8_t onerom_get_cs3_pin(void) { return cs3_pin; }
EPIO_EXPORT uint8_t onerom_get_x1_pin(void) { return x1_pin; }
EPIO_EXPORT uint8_t onerom_get_x2_pin(void) { return x2_pin; }
