// Copyright (C) 2026 Piers Finlayson <piers@piers.rocks>
//
// MIT License

// RP2350 Shared PIO routines

#include "include.h"

#if defined(RP235X)

#if defined(TEST_BUILD)
#define TEST_PIO_C
#endif // TEST_BUILD

#define APIO_LOG_IMPL  1
#include "piodma/piodma.h"

int pio(
    const sdrr_info_t *info,
    const sdrr_rom_set_t *set,
    uint32_t rom_table_addr
) {
    int rc;
    if (set->roms[0]->rom_type == CHIP_TYPE_6116) {
        DEBUG("PIO RAM Mode");
        rc = pioram(info, rom_table_addr);
    } else {
        DEBUG("PIO ROM Mode");
        rc = piorom(info, set, rom_table_addr);
    }

    // Only returns in emulation case.
    return rc;
}

#endif // RP235X