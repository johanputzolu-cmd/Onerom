// Copyright (C) 2026 Piers Finlayson <piers@piers.rocks>
//
// MIT License

// Test stub header file

#if defined(TEST_BUILD)
#if !defined(TEST_STUB_H)
#define TEST_STUB_H

#include "stdio.h"
#include "assert.h"
#include "stdlib.h"
#include "test/SEGGER_RTT.h"

// Function prototypes
extern void stub_log(const char* msg, ...);

// Logging macros
#define STUB_LOG stub_log
#define STUB_ASSERT(X, ...) STUB_LOG(__VA_ARGS__); assert(X)
#define STUB_EXIT(X)        STUB_LOG("Exiting with code %d", X); exit(X)

#define TST_LOG(...) STUB_LOG(__VA_ARGS__)
#if defined(DEBUG_TEST)
#define TST_DBG(...) STUB_LOG(__VA_ARGS__)
#else // !DEBUG_TEST
#define TST_DBG(...)
#endif // DEBUG_TEST


// Externs required by firmware
#define RAM_ROM_TABLE_SIZE (1024 * 1024)
extern uint64_t *get_ram_rom_image_table_aligned();
#if defined(TEST_MAIN_C)
// Allocate twice the required RAM ROM table size, so it can be aligned to
// 512KB (done in preload_rom_image).
uint32_t test_ram_rom_image_table[RAM_ROM_TABLE_SIZE*2/4] = {0};
uint64_t *get_ram_rom_image_table_aligned() {
    uint64_t address = (uint64_t)(uintptr_t)test_ram_rom_image_table;
    address += RAM_ROM_TABLE_SIZE-1;
    address /= RAM_ROM_TABLE_SIZE;
    address = address * RAM_ROM_TABLE_SIZE;
    return (uint64_t *)(uintptr_t)address;
}
#endif
#define _metadata_start onerom_metadata_header
#define _ram_rom_image_start test_ram_rom_image_table
#define _sdrr_runtime_info_ram sdrr_runtime_info

#endif // TEST_STUB_H
#endif // TEST_BUILD