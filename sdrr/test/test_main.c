// Copyright (C) 2026 Piers Finlayson <piers@piers.rocks>
//
// MIT License

// PIO tester program
//
// This works by:
// - Building the firmware with a test stub.
// - Executing the firmware up to the point where it is about to start serving
//   ROM data, at which point it returns to the test code.
// - Checking that limp mode hasn't been entered, and that the PIO SMs have
//   been enabled.
// - Driving the address and approrpriate CS lines to read the entire ROM
//   configuration, checking the data read matches the expected data for each
//   address.
//
// Current limitations:
// - Doesn't support 40 pin ROMs, and may need enhancing for 32 pin support.
// - Doesn't support testing multiple ROM sets - only tests the first ROM set
//   in the configuration.
// - Doesn't support testing multi-ROM sets, or dyanmically banked ROMs.

#define TEST_MAIN_C

#include <stdio.h>

#include <include.h>
#include <apio.h>
#include <epio.h>
#include <test/stub.h>
#include <test/func.h>

static int check_rom_read(
    epio_t *epio,
    uint8_t set_index,
    uint8_t rom_index,
    uint32_t addr
);
static void setup_test_infra(void);
static epio_t *start_epio(void);

// Cycle counts to use for tests.  At 150MHz, each cycle is 6.67ns.
//
// Strictly these are more aggressive than the real world, as in the real
// world we have 2 cycles before reading a changed GPIO state due to meta-
// stability handling.
//
// When explicit meta-stability handling is added to epio, these timings will
// need to be relaxed.

// The CS active/inactive loop generally only takes 3 cycles, but may take up
// to 6 in the 2332 case (i.e. non-contiguous CS pins).
#define TST_CYCLES_BEFORE_START             173 // A random number
#define TST_CYCLES_ADDR_BEFORE_CS_ACTIVE    6   // 40ns (+ CS delay)
#define TST_CYCLES_CS_ACTIVE_TO_DATA_READY  6   // 40ns
#define TST_CYCLES_AFTER_READ               6   // 40ns

// Additional delay required for multi-ROM sets as the address can only be
// validly retrieved after CS has gone active.  This allows for another
// address read -> DMA -> data write chain
#define TST_CYCLES_MULTI_ROM_CS_ACTIVE_TO_DATA_READY 12 // 80ns

static int check_rom_read(
    epio_t *epio,
    uint8_t set_index,
    uint8_t rom_index,
    uint32_t addr
) {
    uint8_t multi_rom = 0;
    if (rom_set[set_index].rom_count > 1) {
        multi_rom = 1;
    }

    // Check the data pins are undriven before starting
    if (are_cs_active_all_high(set_index, rom_index)) {
        sdrr_rom_type_t rom_type = get_rom_type(set_index, rom_index);
        assert(
            rom_type == CHIP_TYPE_2316 ||
            rom_type == CHIP_TYPE_2332 ||
            rom_type == CHIP_TYPE_2364 ||
            rom_type == CHIP_TYPE_23128 ||
            rom_type == CHIP_TYPE_23256 ||
            rom_type == CHIP_TYPE_23512 ||
            rom_type == CHIP_TYPE_231024
        );
    } else {
        check_data_pins_undriven(epio);
    }

    // First step is to drive the address GPIOs with CS inactive.
    uint64_t gpios_driven;
    uint64_t gpio_levels;
    get_gpio_drive(
        set_index,
        rom_index,
        addr,
        0,
        &gpios_driven,
        &gpio_levels
    );
    epio_drive_gpios_ext(epio, gpios_driven, gpio_levels);
    epio_step_cycles(epio, TST_CYCLES_ADDR_BEFORE_CS_ACTIVE);

    // Now set CS active
    get_gpio_drive(
        set_index,
        rom_index,
        addr,
        1,
        &gpios_driven,
        &gpio_levels
    );
    epio_drive_gpios_ext(epio, gpios_driven, gpio_levels);
    //TST_LOG("Address 0x%08X driven with CS active", addr);
    uint32_t delay_cycles = multi_rom ? TST_CYCLES_MULTI_ROM_CS_ACTIVE_TO_DATA_READY : TST_CYCLES_CS_ACTIVE_TO_DATA_READY;
    epio_step_cycles(epio, delay_cycles);

    // Check the data lines are being actively driven
    check_data_pins_driven(epio);

    // Read the data lines
    uint64_t gpio_out = epio_read_gpios_ext(epio);
    uint32_t data = get_byte_from_gpio(gpio_out, 8);

    // Get the expected data byte
    uint8_t expected_data = get_rom_image_data_byte(set_index, rom_index, addr);

    // Test it
    int rc = 0;
    if (data != expected_data) {
        if (get_progress() < 5) {
            TST_LOG("ROM Read Mismatch: 0x%08X expected 0x%02X got 0x%02X", addr, expected_data, data);
        }
        rc = 1;
    }

    // Stop driving GPIOs and step a bit, as it isn't realistic to have the
    // next read drive the address pins immediately.
    get_gpio_drive(
        set_index,
        rom_index,
        -1,
        2,
        &gpios_driven,
        &gpio_levels
    );
    epio_drive_gpios_ext(epio, gpios_driven, gpio_levels);
    epio_step_cycles(epio, TST_CYCLES_AFTER_READ);

    inc_progress();

    return rc;
}

static void setup_test_infra(void) {
    setup_addr_pins();
    setup_data_pins();
    setup_rom_images();
    TST_LOG_FILE_CLEAR();
}

static epio_t *start_epio(void) {
    return epio_from_apio();
}

static int test_set(uint8_t set_index) {
    TST_DBG("Testing ROM set %d", set_index);

    uint8_t set_sel_index = stub_set_sel_image(set_index);
    if (set_sel_index != set_index) {
        TST_LOG("WARNING: Insufficient image select jumpers for set %d, testing set %d instead", set_index, set_sel_index);
    }
    set_index = set_sel_index;

    TST_DBG("Launching firmware");

    // Redirect firmware logging to file
    TST_LOG_TO_FILE();

    // Run the firmware - it will return when it hits the infinite loop.
    firmware_main();

        // Re-instate stdout and close the log file
    TST_LOG_RESET_STDOUT();

    if (limp_mode_value != LIMP_MODE_NONE) {
        TST_LOG("Firmware entered limp mode with pattern %d", limp_mode_value);
        return 1;
    }

    if (_apio_emulated_pio.pios_enabled != 1) {
        TST_LOG("PIO programs were not enabled: %d", _apio_emulated_pio.pios_enabled);
        return 1;
    }

    // Start the PIO emulator to run the PIOs/DMAs
    epio_t *epio = start_epio();
    if (!epio) {
        TST_LOG("Failed to initialize EPIO");
        return 1;
    }

    // Copy from the test_ram_rom_image)_table to epio
    uint64_t *source = get_ram_rom_image_table_aligned();
    epio_sram_set(epio, 0x20000000, (uint8_t *)source, 512*1024);

    // Configure the DMA chain
    uint8_t bit_mode = 8;
    sdrr_rom_type_t rom_type = get_rom_type(set_index, 0);
    if (rom_type == CHIP_TYPE_27C400) {
        bit_mode = 16;
    }
    epio_dma_setup_read_pio_chain(
        epio,
        0,
        1,
        0,
        4,
        2,
        1,
        4,
        bit_mode
    );

    // Step the emulator some cycles before we start
    TST_DBG("Stepping emulator for %d cycles before starting tests", TST_CYCLES_BEFORE_START);
    epio_step_cycles(epio, TST_CYCLES_BEFORE_START);

    // Check the first ROM set image, as this is what the firmware will have
    // loaded due to dummy sel index pins.
    uint8_t rom_index = 0;
    uint8_t rom_count = rom_set[set_index].rom_count;
    uint32_t failures = 0;
    for (rom_index = 0; rom_index < rom_count; rom_index++) {
        const sdrr_rom_info_t *rom = rom_set[set_index].roms[rom_index];
        uint32_t rom_size = get_rom_image_size(set_index, rom_index);
        TST_LOG("Testing ROM read for set %d image %d %s %s %d bytes ...", set_index, rom_index, chip_type[rom->rom_type], rom->filename, rom_size);
        reset_progress();
        for (uint32_t ii = 0; ii < rom_size; ii++) {
            failures += check_rom_read(
                epio,
                set_index,
                rom_index,
                ii
            );
        }
        if (failures == 0) {
            TST_LOG("  read %d bytes successfully", rom_size);
        } else {
            TST_LOG("  ERROR - %d/%d ROM bytes read successfully", rom_size - failures, rom_size);
        }
    }

    epio_free(epio);

    return failures == 0 ? 0 : 1;
}

int main(int argc, char *argv[]) {
    (void)argc;
    (void)argv;

    TST_LOG("One ROM Fire Firmware Tester");
    TST_LOG("%s %s", copyright, author);
    TST_LOG("");
    TST_LOG("Build date/time:   %s", sdrr_info.build_date);
    TST_LOG("Firmware version:  %d.%d.%d", sdrr_info.major_version, sdrr_info.minor_version, sdrr_info.patch_version);
    if (sdrr_info.build_number) {
        TST_LOG("Firmware build:    %d", sdrr_info.build_number);
    }
    TST_LOG("Hardware revision: %s", sdrr_info.hw_rev);
    TST_LOG("-----");

    // Set up the test infrastructure, also does some checking of the captured
    // config to make sure it looks sane.
    setup_test_infra();

    // Now test each set in turn.
    assert(sdrr_info.metadata_header->rom_set_count == SDRR_NUM_SETS);
    int rc;
    for (int ii = 0; ii < SDRR_NUM_SETS; ii++) {
        rc = test_set(ii);
        if (rc != 0) {
            break;
        }
    }

    free_src_rom_images();

    return rc;
}