// Copyright (C) 2025 Piers Finlayson <piers@piers.rocks>
//
// MIT License

// RP2350 PIO/DMA autonomous ROM serving support

#include "include.h"
#include "piorom.h"

// This file contains a completely autonomous PIO and DMA based ROM serving
// implementation.  Once started, the PIO state machines and DMA channels
// serve ROM data in response to external chip select and address lines
// without any further CPU intervention.
//
// The implementation uses three PIO state machines and 2 DMA channels, with
// the following overall operation:
// - PIO SM0 - Chip Select/Output Data Handler
// - PIO SM1 - Address Reader
// - DMA0    - Address Forwarder
// - DMA1    - Data Byte Fetcher
// - PIO SM2 - Data Byte Writer 
//
//         CS active                              Data to Outputs  CS Inactive
//             |                                               ^        |
//             v                                               |        v
// SM0 ----------+---------------------------------------------+--------->+
//     ^         |                                           ^            |
//     |         | IRQ0 Set                       IRQ0 Clear |            |
//     |         v                                           |            |
//     |        SM1 -------> DMA0 ---------> DMA1 --------> SM2 +         |
//     |         |            |               |                 v         |
//     |     Read Addr   Forward Addr   Get Data Byte      Write Data     |
//     |                                                                  v
//     +<---------------------------------------------------------------+-+
//                                                                      |
//                                                                      v
// (Not to scale)                                               Data to Inputs
//
// The detailed operation is as follows:
//
// PIO0 SM0 - CS Handler
//  - (Initially ensures data pins are inputs.)
//  - Monitors the chip select lines.
//  - When all CS lines are active, triggers an IRQ to signal the address
//    read SM to read the address lines.
//  - Waits until the IRQ is cleared by the data byte SM before continuing.
//  - Sets the data pins to outputs.  This is coincident with the data byte
//    output SM serving the data byte.
//  - Tight loops, checking for CS going inactive.
//  - When CS goes inactive again, sets data pins back to inputs and starts
//    over.
//
// PIO0 SM1 - Address Read
//  - (One time - reads high 16 bits of ROM table address from its TX FIFO.
//    This is preloaded by the CPU before starting the PIOs.)
//  - Prepares by pushing high 16 bits of ROM table address into its OSR.
//  - Waits for IRQ from CS Handler SM.
//  - When IRQ received, reads the address lines (16 bits) into OSR, completing
//    the ROM table lookup address for the byte to be served.
//  - Pushes the complete 32 bit ROM table lookup address into its RX FIFO.
//  - Loops back to 2nd step (pushing high 16 bits of ROM table address into
//    OSR), then waits for IRQ again.
//
// DMA Channel 0 - Address Forwarder
//  - Triggered by PIO0 SM1 RX FIFO using DREQ_PIO0_RX1 (SM1 RX FIFO).
//  - Reads the 32 bit ROM table lookup address from PIO0 SM1 RX FIFO.
//  - Writes the address into DMA Channel 1 READ_ADDR register.
//  - Chains to DMA Channel 1.
//
// DMA Channel 1 - Data Byte Fetcher
//  - Triggered by being chained to by DMA1.
//  - Reads the ROM byte from the address specified in its READ_ADDR register.
//  - Writes the byte into PIO0 SM2 TX FIFO.
//  - Chains back to DMA Channel 0, which is then primed to read the next
//    ROM table lookup address from PIO0 SM1 RX FIFO when available.
//
// PIO0 SM2 - Data Byte Output
//  - (One time - sets data pins to low.)
//  - Waits for data byte to be available in its TX FIFO.
//  - When data byte available, clears the CS active IRQ to signal the CS
//    (causing SM0 CS Handler to set data lines as outputs).
//  - Outputs the data byte on the data pins.
//  - Loops back to waiting for next data byte.
//
// There are a number of hardware pre-requisites for this to work correctly:
// - RP2350 (vs RP2040, as this implementation uses pinsdirs as a mov
//   destination).
// - All Chip Select (or CE/OE) lines must be connected to contiguous GPIOs.
// - Any active high chip seledct lines must be inverted using GPIO input
//   inversion (INOVER).
// - All Data lines must be connected to contiguous GPIOs.
// - All Address lines must be connected to contiguous GPIOs, and be limited
//   to a 64KB address space.  (Strictly other powers of two could be
//   supported.)
//
// In order to minimise jitter, it is advisable to ensure the following:
// - The DMA channels have high AHB5 bus priority for both reads
//   and writes using the BUS_PRIORITY register.
// - Nothing else attempts to read or write to the 4 banks of SRAM the
//   64KB ROM table is striped across.
// - If other DMAs are enabled, the DMAs within this module should have a
//   higher priority set.
// - Nothing else accesses peripherals on the AHB5 splitter during operation.
//
// Possible enhancements:
// - May want to check CS is still active before setting data pins to outputs
//   in SM2.
// - May need to add delays, e.g. before reading address lines to allow them
//   stabilise.  Ideally, we would make side-set delays configurable at
//   various points in the algorithm, and include as configuration options.

//
// PIO state machine programs
//

// PIO Programs are currently hand generated using `pioasm` and copied into
// this source file.

// CS Handler - SM0
#define ONEROM_CS_HANDLER_START         0
#define ONEROM_CS_HANDLER_WRAP_BOTTOM   0
#define ONEROM_CS_HANDLER_WRAP_TOP      6
#define ONEROM_CS_HANDLER_LEN           7
static const uint16_t onerom_cs_handler[] = {
            //     .wrap_target
    0xa063, //  0: mov    pindirs, null  - set data pins to inputs
    0xa020, //  1: mov    x, pins        - read CS lines
    0x0041, //  2: jmp    x--, 1         - CS inactive, loop back to re-read CS
    0xc020, //  3: irq    wait 0         - signal CS active, wait for data byte
    0xa06b, //  4: mov    pindirs, ~null - set data pins to output
    0xa020, //  5: mov    x, pins        - read CS lines again
    0x0025, //  6: jmp    !x, 5         - CS still active?
            //     .wrap
};

// Address Read - SM1
#define ONEROM_ADDR_READ_START          0
#define ONEROM_ADDR_READ_WRAP_BOTTOM    2
#define ONEROM_ADDR_READ_WRAP_TOP       4
#define ONEROM_ADDR_READ_LEN            5
static const uint16_t onerom_addr_read[] = {
    0x80a0, //  0: pull   block          - get high word of ROM table address
    0xa027, //  1: mov    x, osr         - store high word in X
            //     .wrap_target
    0x4030, //  2: in     x, 16          - read high address bits from X
    0x20c0, //  3: wait   1 irq, 0       - wait for CS to go active
    0x4010, //  4: in     pins, 16       - read address lines (autopush)
            //     .wrap
};

// Data Byte Output - SM2
#define ONEROM_DATA_BYTE_START          0
#define ONEROM_DATA_BYTE_WRAP_BOTTOM    1
#define ONEROM_DATA_BYTE_WRAP_TOP       3
#define ONEROM_DATA_BYTE_LEN            4
static const uint16_t onerom_data_byte[] = {
    0xa003, //  0: mov    pins, null     - set data pins to low
            //     .wrap_target
    0x80a0, //  1: pull   block          - get data byte from TX FIFO (waits)
    0xc040, //  2: irq    clear 0        - clear CS active IRQ - triggers data pins to outputs
    0x6008, //  3: out    pins, 8        - signal data byte on data pins
            //     .wrap
};

//
// PIO and DMA Configuration
//

#define NUM_DATA_LINES    8
#define NUM_ADDR_LINES    16

// Loads the PIO programs into the PIO instruction memory.
void piorom_load_programs(
    uint8_t num_cs_pins,
    uint8_t cs_base_pin,
    uint8_t data_base_pin,
    uint8_t addr_base_pin,
    uint32_t rom_table_addr
) {
    volatile pio_sm_reg_t *sm_reg;
    uint8_t offset = 0;

    // Clear all PIO0 IRQs
    PIO0_IRQ = 0x000000FF;

    // SM0 - CS handler

    // Load the CS handler program
    uint8_t sm0_start = offset;
    for (int ii = 0; ii < ONEROM_CS_HANDLER_LEN; ii++, offset++) {
        PIO0_INSTR_MEM(offset) = onerom_cs_handler[ii];
    }

    // Configure the CS handler SM
    sm_reg = PIO0_SM_REG(0);
    sm_reg->clkdiv = PIO_CLKDIV_INT(1, 0);
    sm_reg->execctrl =
        PIO_WRAP_BOTTOM(sm0_start + ONEROM_CS_HANDLER_WRAP_BOTTOM) |
        PIO_WRAP_TOP(sm0_start + ONEROM_CS_HANDLER_WRAP_TOP);
    sm_reg->shiftctrl = PIO_IN_COUNT(num_cs_pins);
    sm_reg->pinctrl =
        PIO_OUT_COUNT(NUM_DATA_LINES) | 
        PIO_OUT_BASE(data_base_pin) |
        PIO_IN_BASE(cs_base_pin);
    sm_reg->instr = PIO_INST_JMP_UNCOND(sm0_start + ONEROM_CS_HANDLER_START);

    // SM1 - Address read

    // Load the address read program
    uint8_t sm1_start = offset;
    for (int ii = 0; ii < ONEROM_ADDR_READ_LEN; ii++, offset++) {
        PIO0_INSTR_MEM(offset) = onerom_addr_read[ii];
    }

    // Configure the address read SM
    sm_reg = PIO0_SM_REG(1);
    sm_reg->clkdiv = PIO_CLKDIV_INT(1, 0);
    sm_reg->execctrl = 
        PIO_WRAP_BOTTOM(sm1_start + ONEROM_ADDR_READ_WRAP_BOTTOM) |
        PIO_WRAP_TOP(sm1_start + ONEROM_ADDR_READ_WRAP_TOP);
    sm_reg->shiftctrl =
        PIO_IN_COUNT(NUM_ADDR_LINES) |
        PIO_AUTOPUSH |
        PIO_PUSH_THRESH(32) |
        PIO_IN_SHIFTDIR_L |
        PIO_OUT_SHIFTDIR_L;
    sm_reg->pinctrl =
        PIO_IN_BASE(addr_base_pin);
    sm_reg->instr = PIO_INST_JMP_UNCOND(sm1_start + ONEROM_ADDR_READ_START);

    // Preload the ROM table address into the TX FIFO
    PIO0_SM_TXF(1) = (rom_table_addr >> 16) & 0xFFFF;

    // Configure the address read SM

    // SM2 - Data byte output
    uint8_t sm2_start = offset;
    for (int ii = 0; ii < ONEROM_DATA_BYTE_LEN; ii++, offset++) {
        PIO0_INSTR_MEM(offset) = onerom_data_byte[ii];
    }

    // Configure the data byte SM
    sm_reg = PIO0_SM_REG(2);
    sm_reg->clkdiv = PIO_CLKDIV_INT(1, 0);
    sm_reg->execctrl = 
        PIO_WRAP_BOTTOM(sm2_start + ONEROM_DATA_BYTE_WRAP_BOTTOM) |
        PIO_WRAP_TOP(sm2_start + ONEROM_DATA_BYTE_WRAP_TOP);
    sm_reg->shiftctrl =
        //PIO_AUTOPULL |
        //PIO_PULL_THRESH(NUM_DATA_LINES) |
        PIO_OUT_SHIFTDIR_R;
    sm_reg->pinctrl =
        PIO_OUT_BASE(data_base_pin) |
        PIO_OUT_COUNT(NUM_DATA_LINES);
    sm_reg->instr = PIO_INST_JMP_UNCOND(sm2_start + ONEROM_DATA_BYTE_START);
}

// Starts the PIO state machines for ROM serving.
void piorom_start_pios() {
    PIO0_CTRL_SM_ENABLE(0x7); // Enable SM0, SM1 and SM2
}

// Set GPIOs to PIO function for ROM serving
//
// cs_pin_invert is an array of uint8_t with 1 meaning active high
void piorom_set_gpio_func(
    uint8_t num_cs_pins,
    uint8_t cs_base_pin,
    uint8_t cs_pin_invert[],
    uint8_t data_base_pin,
    uint8_t addr_base_pin
) {
    // CS pins
    for (int ii = 0; ii < num_cs_pins; ii++) {
        uint8_t pin = cs_base_pin + ii;
        uint8_t invert = cs_pin_invert[ii];
        if (!invert) {
            GPIO_CTRL(pin) = GPIO_CTRL_FUNC_PIO0;
        } else {
            // Turn CS line into active low
            GPIO_CTRL(pin) = GPIO_CTRL_FUNC_PIO0 | GPIO_CTRL_INOVER_INVERT;
        }
    }

    // Data pins
    for (int ii = data_base_pin;
        ii < (data_base_pin + NUM_DATA_LINES);
        ii++) {
        GPIO_CTRL(ii) = GPIO_CTRL_FUNC_PIO0;
    }

    // Address pins
    for (int ii = addr_base_pin;
        ii < (addr_base_pin + NUM_ADDR_LINES);
        ii++) {
        GPIO_CTRL(ii) = GPIO_CTRL_FUNC_PIO0;
    }
}

// Setup the DMA channels for ROM serving
void piorom_setup_dma(
    uint8_t pio_block,
    uint8_t sm_addr_read,
    uint8_t sm_data_byte
) {
    volatile dma_ch_reg_t *dma_reg;

    // DMA Channel 0 - Receives ROM table lookup address from PIO0 SM1 and
    // sends it onto DMA Channel 1.  Triggered by PIO0 SM1 RX FIFO DREQ.
    dma_reg = DMA_CH_REG(0);
    dma_reg->read_addr = (uint32_t)&PIO0_SM_RXF(sm_addr_read);
    dma_reg->write_addr = (uint32_t)&DMA_CH_READ_ADDR(1);
    dma_reg->transfer_count = 1;
    dma_reg->ctrl_trig =
        DMA_CTRL_TRIG_TREQ_SEL(DREQ_PIO_X_SM_Y_RX(pio_block, sm_addr_read)) |
        DMA_CTRL_TRIG_CHAIN_TO(1) |
        DMA_CTRL_TRIG_EN |
        DMA_CTRL_TRIG_DATA_SIZE_32BIT;

    // DMA Channel 1 - Reads ROM data from memory and sends to PIO0 SM2.
    // Triggered by being chained to by DMA Channel 0.
    dma_reg = DMA_CH_REG(1);
    dma_reg->read_addr = 0; // To be set by DMA Channel 0
    dma_reg->write_addr = (uint32_t)&PIO0_SM_TXF(sm_data_byte);
    dma_reg->transfer_count = 0x1;
    dma_reg->ctrl_trig =
        DMA_CTRL_TRIG_TREQ_SEL(DMA_CTRL_TRIG_TREQ_PERM) |
        DMA_CTRL_TRIG_CHAIN_TO(0) |
        DMA_CTRL_TRIG_EN |
        DMA_CTRL_TRIG_DATA_SIZE_8BIT;

    // Set DMA Read as high priority on the AHB5 bus for both:
    // - Reads (from RAM and PIO RX FIFO)
    // - Writes (to PIO TX FIFO and DMA READ_ADDR)
    BUSCTRL_BUS_PRIORITY |=
        BUSCTRL_BUS_PRIORITY_DMA_R_BIT |
        BUSCTRL_BUS_PRIORITY_DMA_W_BIT;
}

// Get lowest data GPIO from the pin info
uint8_t get_lowest_data_gpio(
    const sdrr_info_t *info
) {
    uint8_t lowest = MAX_USED_GPIOS;
    for (int ii = 0; ii < 8; ii++) {
        if (info->pins->data[ii] < lowest) {
            lowest = info->pins->data[ii];
        }
    }
    return lowest;
}

// Get lowest address GPIO from the pin info
uint8_t get_lowest_addr_gpio(
    const sdrr_info_t *info
) {
    uint8_t lowest = MAX_USED_GPIOS;
    for (int ii = 0; ii < 16; ii++) {
        if (info->pins->addr[ii] < lowest) {
            lowest = info->pins->addr[ii];
        }
    }
    return lowest;
}

// Configure and start the Autonomous PIO/DMA ROM serving implementation.
void piorom(
    const sdrr_info_t *info,
    const sdrr_rom_set_t *set,
    uint32_t rom_table_addr
) {
    (void)set;

    // Get lowest data and address GPIOs
    uint8_t get_lowest_data_gpio_val = get_lowest_data_gpio(info);
    uint8_t get_lowest_addr_gpio_val = get_lowest_addr_gpio(info);

    // Bring PIO0 and DMA out of reset
    RESET_RESET &= ~(RESET_PIO0 | RESET_DMA);
    while (!(RESET_DONE & (RESET_PIO0 | RESET_DMA)));

    // Setup the DMA channels:
    // - PIO block 0
    // - SM1 is the address read SM
    // - SM2 is the data byte output SM
    piorom_setup_dma(0, 1, 2);

    // Hard code CS handling for for now
    uint8_t cs_pin_invert[2] = {0, 1}; // CS0 active low, CS1 active high

    // Configure GPIOs for PIO function
    // - 2 CS pins
    // - CS pins start at GPIO 10
    // - CS active high/low config
    // - Data pins start at GPIO 0
    // - Address pins start at GPIO 8
    piorom_set_gpio_func(
        1,
        info->pins->cs1,
        cs_pin_invert,
        get_lowest_data_gpio_val,
        get_lowest_addr_gpio_val
    );

    // Load and configure the PIO programs
    // - 2 CS pins
    // - CS pins start at GPIO 10
    // - Data pins start at GPIO 0
    // - Address pins start at GPIO 8
    piorom_load_programs(
        1,
        info->pins->cs1,
        get_lowest_data_gpio_val,
        get_lowest_addr_gpio_val,
        rom_table_addr
    );

    // Start the PIOs.  This kicks off the autonomous ROM serving.
    piorom_start_pios();

    while (1) {
        // Low power wait for (VBUS) interrupt.  Avoids any potential SRAM or
        // peripheral access that might introduce jitter on the PIO/DMA
        // serving.
        __asm volatile("wfi");
    }
}

// Notes:
// - Hardcoded to 1 CS pin at GPIO10, active low - need to fix active high