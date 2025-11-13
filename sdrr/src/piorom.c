// Copyright (C) 2025 Piers Finlayson <piers@piers.rocks>
//
// MIT License

// RP2350 PIO/DMA autonomous ROM serving support

#include "include.h"
#include "piorom.h"

// PIO Programs are currently hand generated using `pioasm` and copied into
// this source file:
//
// ~/builds/pico-sdk/build/pioasm/pioasm -v 1 -o c-sdk sdrr/src/rom.pio > /tmp/pio.c

// PIO state machine programs
#define ONEROM_CS_HANDLER_START         0
#define ONEROM_CS_HANDLER_WRAP_BOTTOM   0
#define ONEROM_CS_HANDLER_WRAP_TOP      6
#define ONEROM_CS_HANDLER_LEN           7
static const uint16_t onerom_cs_handler[] = {
            //     .wrap_target
    0xa063, //  0: mov    pindirs, null
    0xa020, //  1: mov    x, pins
    0x0024, //  2: jmp    !x, 4
    0x0001, //  3: jmp    1
    0xa06b, //  4: mov    pindirs, ~null
    0xa020, //  5: mov    x, pins
    0x0025, //  6: jmp    !x, 5
            //     .wrap
};
#define ONEROM_ADDR_READ_START          0
#define ONEROM_ADDR_READ_WRAP_BOTTOM    2
#define ONEROM_ADDR_READ_WRAP_TOP       3
#define ONEROM_ADDR_READ_LEN            4
static const uint16_t onerom_addr_read[] = {
    0x80a0, //  0: pull   block
    0xa027, //  1: mov    x, osr
            //     .wrap_target
    0x4030, //  2: in     x, 16
    0x4110, //  3: in     pins, 16               [1]
            //     .wrap
};
#define ONEROM_DATA_BYTE_START          0
#define ONEROM_DATA_BYTE_WRAP_BOTTOM    0
#define ONEROM_DATA_BYTE_WRAP_TOP       1
#define ONEROM_DATA_BYTE_LEN            2
static const uint16_t onerom_data_byte[] = {
    0xa003, //  0: mov    pins, null
            //     .wrap_target
    0x6008, //  1: out    pins, 8
            //     .wrap
};

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
        PIO_AUTOPULL |
        PIO_PULL_THRESH(NUM_DATA_LINES) |
        PIO_OUT_SHIFTDIR_L;
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
    uint8_t *invert = cs_pin_invert;
    for (int ii = cs_base_pin;
        ii < (cs_base_pin + num_cs_pins);
        ii++, invert++) {
        if (!(*invert)) {
            GPIO_CTRL(ii) = GPIO_CTRL_FUNC_PIO0;
        } else {
            // Turn CS line into active low
            GPIO_CTRL(ii) = GPIO_CTRL_FUNC_PIO0 | GPIO_CTRL_INOVER_INVERT;
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
    // sends it onto DMA Channel 1.  Triggered by PIO0 SM1 RX FIFO.
    dma_reg = DMA_CH_REG(0);
    dma_reg->read_addr = (uint32_t)&PIO0_SM_RXF(sm_addr_read);
    dma_reg->write_addr = (uint32_t)&DMA_CH_READ_ADDR_TRIG(1);
    dma_reg->transfer_count = 1;
    dma_reg->ctrl_trig =
        DMA_CTRL_TRIG_TREQ_SEL(DREQ_PIO_X_SM_Y_RX(pio_block, sm_addr_read)) |
        DMA_CTRL_TRIG_CHAIN_TO(0) |
        DMA_CTRL_TRIG_EN |
        DMA_CTRL_TRIG_DATA_SIZE_32BIT;

    // DMA Channel 1 - Reads ROM data from memory and sends to PIO0 SM2.
    // Triggered by writes to its READ_ADDR_TRIG register by DMA Channel 0.
    dma_reg = DMA_CH_REG(1);
    dma_reg->read_addr = 0; // To be set by DMA Channel 0
    dma_reg->write_addr = (uint32_t)&PIO0_SM_TXF(sm_data_byte);
    dma_reg->transfer_count = 0x1;
    dma_reg->ctrl_trig =
        DMA_CTRL_TRIG_TREQ_SEL(DMA_CTRL_TRIG_TREQ_PERM) |
        DMA_CTRL_TRIG_CHAIN_TO(1) |
        DMA_CTRL_TRIG_EN |
        DMA_CTRL_TRIG_DATA_SIZE_8BIT;
}

// Configure the PIO ROM serving programs
void piorom(void) {
    // Bring PIO0 and DMA out of reset
    RESET_RESET &= ~(RESET_PIO0 | RESET_DMA);
    while (!(RESET_DONE & (RESET_PIO0 | RESET_DMA)));

    // Setup the DMA channels
    piorom_setup_dma(0, 1, 2);

    // Hard code for now
    uint8_t cs_pin_invert[1] = {0};
    piorom_set_gpio_func(
        1,
        10,
        cs_pin_invert,
        0,
        8
    );
    piorom_load_programs(
        1,
        10,
        0,
        8,
        0x20000000
    );

    // Start the PIOs
    piorom_start_pios();

    while (1) {
        // Do nothing - PIO/DMA handles everything
        ;
    }
}