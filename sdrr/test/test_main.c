// Copyright (C) 2026 Piers Finlayson <piers@piers.rocks>
//
// MIT License

// Main test program

#define TEST_MAIN_C

#include <stdio.h>
#if !defined(EPIO_WASM)
#include <unistd.h>
#endif // EPIO_WASM

#include <include.h>
#include <test/stub.h>
#include <apio.h>
#include <epio.h>

static uint8_t addr_pins[32];
static uint8_t data_pins[16];

static void check_rom_read(
    epio_t *epio,
    uint32_t addr,
    uint8_t num_addr_bits,
    uint8_t cs1,
    uint8_t cs2,
    uint8_t cs3,
    uint8_t x1,
    uint8_t x2,
    uint32_t expected_data
);
static uint32_t get_byte_from_gpio(uint64_t gpio_in, uint8_t data_bits);
static void get_gpio_drive_from_addr_cs(
    uint32_t addr,
    uint8_t num_addr_bits,
    uint8_t cs1,
    uint8_t cs2,
    uint8_t cs3,
    uint8_t x1,
    uint8_t x2,
    uint64_t *gpios_to_drive,
    uint64_t *gpio_levels
);
static void setup_addr_pins(void);
static void setup_data_pins(void);
static void setup_test_infra(void);
static epio_t *start_epio(void);

static void check_rom_read(
    epio_t *epio,
    uint32_t addr,
    uint8_t num_addr_bits,
    uint8_t cs1,
    uint8_t cs2,
    uint8_t cs3,
    uint8_t x1,
    uint8_t x2,
    uint32_t expected_data
) {
    // Do the read
    uint64_t gpio_driven;
    uint64_t gpio_level;
    get_gpio_drive_from_addr_cs(
        addr, 
        num_addr_bits,
        cs1,
        cs2,
        cs3,
        x1,
        x2,
        &gpio_driven,
        &gpio_level
    );
    epio_drive_gpios_ext(epio, gpio_driven, gpio_level);
    
    // Step enough to through the entire chain
    epio_step_cycles(epio, 20);

    // Read the data
    uint64_t gpio_out = epio_read_gpios_ext(epio);
    uint32_t data = get_byte_from_gpio(gpio_out, 8);

    if (data != expected_data) {
        TST_LOG("ROM Read Mismatch: 0x%08X expected 0x%02X got 0x%02X", addr, expected_data, data);
        assert(0 && "ROM read mismatch");
    }

    // Stop driving GPIOs
    gpio_driven = 0;
    epio_drive_gpios_ext(epio, 0, 0);
}

// Turns a GPIO read into an actual, de-mangled, data byte
static uint32_t get_byte_from_gpio(uint64_t gpio_in, uint8_t data_bits) {
    uint32_t data = 0;

    assert(((data_bits == 8) || (data_bits == 16)) && "Invalid number of data bits");

    for (int ii = 0; ii < data_bits; ii++) {
        assert((data_pins[ii] < MAX_USED_GPIOS) && "Data bit out of range for ROM");
        if (gpio_in & (1ULL << data_pins[ii])) {
            data |= (1 << ii);
        }
    }
    return data;
}

static void setup_test_infra(void) {
    setup_addr_pins();
    setup_data_pins();
}

static epio_t *start_epio(void) {
    return epio_from_apio();
}

// Gets the appropriate GPIO drive state to simulate a particular address and
// CS state being driven on the ROM by the host.  Returns bit masks ready to
// be applied via epio_drive_gpios_ext.
static void get_gpio_drive_from_addr_cs(
    uint32_t addr,
    uint8_t num_addr_bits,
    uint8_t cs1,
    uint8_t cs2,
    uint8_t cs3,
    uint8_t x1,
    uint8_t x2,
    uint64_t *gpios_to_drive,
    uint64_t *gpio_levels
) {
    assert(addr < (512*1024) && "Address out of range for One ROM");
    uint64_t drive_mask = 0;
    uint64_t level_mask = 0;

    // Figure out the drive and level mask for the address lines
    for (int ii = 0; ii < num_addr_bits; ii++) {
        assert((addr_pins[ii] < MAX_USED_GPIOS) && "Address bit out of range for ROM");
        drive_mask |= (1ULL << addr_pins[ii]);
        if (addr & (1 << ii)) {
            level_mask |= (1ULL << addr_pins[ii]);
        }
    }

    // Add CS lines to the drive and level mask
    if (cs1 < 2) {
        drive_mask |= (1ULL << sdrr_info.pins->cs1);
        if (cs1) {
            level_mask |= (1ULL << sdrr_info.pins->cs1);
        }
    }
    if (cs2 < 2) {
        drive_mask |= (1ULL << sdrr_info.pins->cs2);
        if (cs2) {
            level_mask |= (1ULL << sdrr_info.pins->cs2);
        }
    }
    if (cs3 < 2) {
        drive_mask |= (1ULL << sdrr_info.pins->cs3);
        if (cs3) {
            level_mask |= (1ULL << sdrr_info.pins->cs3);
        }
    }
    if (x1 < 2) {
        drive_mask |= (1ULL << sdrr_info.pins->x1);
        if (x1) {
            level_mask |= (1ULL << sdrr_info.pins->x1);
        }
    }
    if (x2 < 2) {
        drive_mask |= (1ULL << sdrr_info.pins->x2);
        if (x2) {
            level_mask |= (1ULL << sdrr_info.pins->x2);
        }
    }

    *gpios_to_drive = drive_mask;
    *gpio_levels = level_mask;
}

static void setup_addr_pins(void) {
    for (int ii = 0; ii < 16; ii++) {
        addr_pins[ii] = sdrr_info.pins->addr[ii];
    }
    for (int ii = 0; ii < 8; ii++) {
        addr_pins[16 + ii] = sdrr_info.pins->addr2[ii];
    }
}

static void setup_data_pins(void) {
    for (int ii = 0; ii < 8; ii++) {
        data_pins[ii] = sdrr_info.pins->data[ii];
    }
    for (int ii = 0; ii < 8; ii++) {
        data_pins[8 + ii] = sdrr_info.pins->data2[ii];
    }
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
    TST_LOG("");
    TST_LOG("Launching firmware");
    TST_LOG("-----");

    // Set up the test infrastructure, also does some checking of the captured
    // config to make sure it looks sane.
    setup_test_infra();

#if !defined(EPIO_WASM)
    // Redirect firmware logging to file
    char *firmware_log_file = "/tmp/onerom-firmware-output.txt";
    TST_LOG("Firmware logging: %s", firmware_log_file);
    int saved_stdout = dup(STDOUT_FILENO);
    FILE *file = fopen(firmware_log_file, "w");
    if (!file) {
        perror("Failed to open firmware log file");
        return 1;
    }
    dup2(fileno(file), STDOUT_FILENO);
#endif // EPIO_WASM

    // Run the firmware - it will return when it hits the infinite loop.
    firmware_main();

#if !defined(EPIO_WASM)
    // Re-instate stdout and close the log file
    dup2(saved_stdout, STDOUT_FILENO);
    close(saved_stdout);
    fclose(file);
#endif // EPIO_WASM

    TST_LOG("-----");

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
    uint32_t *source = get_ram_rom_image_table_aligned();
    epio_sram_set(epio, 0x20000000, (uint8_t *)source, 512*1024);

    // Check a 2364 (8KB) ROM
    int rom_size = 0x2000;
    for (int ii = 0; ii < rom_size; ii++) {
        check_rom_read(
            epio,
            ii,
            13,
            0,
            0,
            0,
            0,
            0,
            ii & 0x3F
        );
        epio_step_cycles(epio, 20);
    }
    TST_LOG("Read 0x%08X bytes successfully", rom_size);

    epio_free(epio);

    return 0;
}