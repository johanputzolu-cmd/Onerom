// Copyright (C) 2026 Piers Finlayson <piers@piers.rocks>
//
// MIT License

// RP2350 PIO/DMA autonomous RAM serving support

#include "include.h"

#if defined(RP235X)

#include "piodma/piodma.h"

//
// Config options
//

// Number of checks to confirm /W is active.  Can we used to debounce noisy /W
// signals, or brief /W low glitches.
#define PIORAM_WE_ACTIVE_CHECK_MAX 8  // To high and we'll run out of instructions
#define PIORAM_WE_ACTIVE_CHECK_MIN 1
#ifndef PIORAM_WE_ACTIVE_CHECK_COUNT
#define PIORAM_WE_ACTIVE_CHECK_COUNT 2
#endif // PIORAM_WE_ACTIVE_CHECK_COUNT

#ifndef PIORAM_WE_ACTIVE_IRQ_DELAY
// Number of cycles to delay after triggering RAM WRITE IRQ before checking
// whether /W has gone high.  This provides time for the data and address
// reader SMs to get into a state where they can check /W as well.
#define PIORAM_WE_ACTIVE_IRQ_DELAY 4
#endif // PIORAM_WE_ACTIVE_IRQ_DELAY

// Configuration structure for PIO RAM serving
typedef struct pioram_config {
    // CS pin configuration for READ (/CE and /OE)
    uint8_t read_cs_base_pin;
    uint8_t num_read_cs_pins;  // Should be 2 for 6116
    
    // CS pin configuration for WRITE (/CE and /W)
    uint8_t write_cs_base_pin;
    uint8_t num_write_cs_pins;  // Should be 2 for 6116
    
    // Data pins (Q0-Q7)
    uint8_t data_base_pin;
    uint8_t num_data_pins;  // 8 for 6116
    
    // Address pins (A0-A10)
    uint8_t addr_base_pin;
    uint8_t num_addr_pins;  // 11 for 6116 (2KB)
    
    // RAM table base address in SRAM
    uint32_t ram_table_addr;
    
    // Clock dividers for each SM
    uint16_t read_cs_clkdiv_int;
    uint8_t read_cs_clkdiv_frac;
    uint8_t pad0;
    
    uint16_t write_cs_clkdiv_int;
    uint8_t write_cs_clkdiv_frac;
    uint8_t pad1;
    
    uint16_t addr_clkdiv_int;
    uint8_t addr_clkdiv_frac;
    uint8_t pad2;
    
    uint16_t data_out_clkdiv_int;
    uint8_t data_out_clkdiv_frac;
    uint8_t pad3;
    
    uint16_t data_in_clkdiv_int;
    uint8_t data_in_clkdiv_frac;
    uint8_t pad4;
} pioram_config_t;

// Function prototypes
static void pioram_load_programs(pioram_config_t *config);
static void pioram_setup_dma(pioram_config_t *config);
static void pioram_set_gpio_func(pioram_config_t *config);
static void pioram_start_pios(void);

// Things I might be able to remove:
// - Padding in RAM WRITE addr (P1/1) and data handlers (P2/S2) to sync with each other (commented out)
// - Padding in RAM WRITE triggerer (P2/S3) to avoid detecting /W high before the other SMs can (commented out)
// - Extra /CE/W check at start of RAM WRITE triggerer (P2/S3)

// Load all PIO programs for RAM serving
static void pioram_load_programs(pioram_config_t *config) {
    PIO_INSTR_SCRATCH;
    volatile pio_sm_reg_t *sm_reg;
    
    // Clear all PIO IRQs
    PIO_CLEAR_ALL_IRQS();

    uint32_t ram_table_high_bits = (config->ram_table_addr >> 16) & 0xFFFF;
    DEBUG("RAM table high 16 bits: 0x%08X", ram_table_high_bits);

    // PIO0 Programs
    //
    // Combined data/address handlers
    //
    PIO_OFFSET(0);

    //
    // SM0 - Data read handler - triggers data read chain on /CE and /W low
    //
    //
    // Reads both /CE and /W together.  When both are low, triggers first the
    // WRITE address reader, then the data input reader.
    //
    // Re-arms once either /CE or /W goes high.
    PIO_SM_VARS(0, 0);

    PIO_SM_VAR_NEW(0, 0, start_write_enabled_check);
    // This algorithm will check /CE and /W this number of times when it goes
    // low, to make sure it's really low.
    uint8_t data_read_check_count = PIORAM_WE_ACTIVE_CHECK_COUNT;
    if (data_read_check_count > PIORAM_WE_ACTIVE_CHECK_MAX) {
        data_read_check_count = PIORAM_WE_ACTIVE_CHECK_MAX;
        LOG("!!! PIORAM WE ACTIVE CHECK COUNT too high, limiting to %d", PIORAM_WE_ACTIVE_CHECK_MAX);
    } else if (data_read_check_count < PIORAM_WE_ACTIVE_CHECK_MIN) {
        data_read_check_count = 1;
        LOG("!!! PIORAM WE ACTIVE CHECK COUNT too low, setting to 1");
    }
    for (int ii = 0; ii < data_read_check_count; ii++) {
        // Read /CE and /W
        PIO_ADD_INSTR(0, MOV_X_PINS);
        
        // If either /CE or /W is high, check again
        PIO_ADD_INSTR(0, JMP_X_DEC(PIO_SM_VAR(0, 0, start_write_enabled_check)));
    }

    // Trigger RAM WRITE IRQ. Triggers both addr and data readers
    PIO_ADD_INSTR(0, ADD_DELAY(
        IRQ_SET(3),
        PIORAM_WE_ACTIVE_IRQ_DELAY
    )); 

    // Wait for either /CE or /W to go high
    PIO_SM_VAR_NEW(0, 0, check_write_disabled);
    PIO_ADD_INSTR(0, MOV_X_PINS);

    // If both /CE or /W still low, keep waiting, otherwise jump to start
    PIO_SM_SET_WRAP_TOP(0, 0);
    PIO_ADD_INSTR(0, JMP_NOT_X(PIO_SM_VAR(0, 0, check_write_disabled)));

    // Set the various SM register values
    PIO_SM_CLKDIV_SET(0, 0, 1, 0);
    PIO_SM_EXECCTRL_SET(0, 0, 0);
    PIO_SM_SHIFTCTRL_SET(0, 0,
        PIO_IN_COUNT(2) |    // Reading /CE and /W
        PIO_IN_SHIFTDIR_L
    );
    PIO_SM_PINCTRL_SET(0, 0, 
        PIO_IN_BASE(11)  // /CE and /W pins
    );

    // Commit them and jmp to start of this SM
    PIO_SM_COMMIT_REGS(0, 0);
    PIO_SM_JMP_TO_START(0, 0);

    // End of block
    PIO_WRITE_BLOCK(0);
    // Log each SM
    PIO_LOG_SM(0, 0, "Trigger Data and Address Reader (RAM WRITE)");

    //
    // PIO1 Programs
    //
    // Address Readers
    //
    uint8_t offset = 0;

    //
    // PIO1 - Address Readers
    // 
    // SM0 - Address Reader (RAM READ)
    //
    // Constantly serves bytes to the READ DMA chain
    //
    
    // Preload high 16 bits of RAM table address to X - done via TX FIFO
    // before starting as SET(X) only supports 5 bits.

    // Pull high 16 bits from X
    uint8_t addr_wrap_start = offset;
    uint8_t addr_wrap_1st_instr = offset;
    uint8_t addr_wrap_bottom = offset;
    instr_scratch[offset++] = IN_X(16);

    // Read address lines and push to RX FIFO, so READ DMA chain serves the
    // byte.
    // We add a delay after this, to avoid overloading the DMA chain.
    uint8_t addr_wrap_top = offset;
    instr_scratch[offset++] = ADD_DELAY(IN_PINS(16), 2);  // Autopush

    sm_reg = PIO1_SM_REG(0);
    sm_reg->clkdiv = PIO_CLKDIV(1, 0);
    sm_reg->execctrl = 
        PIO_WRAP_BOTTOM(addr_wrap_bottom) |
        PIO_WRAP_TOP(addr_wrap_top);
    sm_reg->shiftctrl =
        PIO_IN_COUNT(11) |
        PIO_AUTOPUSH |
        PIO_PUSH_THRESH(32) |
        PIO_IN_SHIFTDIR_L |
        PIO_OUT_SHIFTDIR_L;
    sm_reg->pinctrl = PIO_IN_BASE(13);  // Address base pin
    PIO1_SM_TXF(0) = ram_table_high_bits;
    sm_reg->instr = PULL_BLOCK;
    sm_reg->instr = MOV_X_OSR;
    sm_reg->instr = JMP(addr_wrap_start);

    //
    // PIO1 - Address Readers
    //
    // SM1 - Address Reader (RAM WRITE)
    //
    // Wait for Data read handler to trigger via IRQ - this indicates /CE and
    // /W went low.
    //
    // Loop reading the address until /W goes high.
    //
    // When /W goes high, push the last read address to the RX FIFO.  This
    // triggers the WRITE DMA chain.
    //
    // The data reader SM is triggered at the same time (actually one cycle
    // later), runs independently , and similarly waits for /W to go high.  As
    // they are both started at around the same time, and take roughly the same
    // time to loop, the data to write should be in the WRITE DMA chain by the
    // time the DMA gets the address and writes the byte.
    //

    // Preload high 16 bits of RAM table address to X - done via TX FIFO
    // before starting as SET(X) only supports 5 bits.

    // (SM does not start here.). Push combined RAM table address and lower
    // order address bits when /W goes high.
    uint8_t addr_write_valid = offset;
    uint8_t addr_write_1st_instr = offset;
    instr_scratch[offset++] = PUSH_BLOCK;

    // Wait for address reader IRQ from Data read handler
    uint8_t addr_write_start = offset;
    instr_scratch[offset++] = WAIT_IRQ_HIGH_PREV(3);  // Wait for RAM WRITE IRQ

    // Pull high 16 bits from X
    uint8_t addr_write_wrap_bottom = offset;
    instr_scratch[offset++] = IN_X(16);

    // Read address lines.
    instr_scratch[offset++] = IN_PINS(16);

    uint8_t addr_write_wrap_top = offset;
    instr_scratch[offset++] = JMP_PIN(addr_write_valid);  // Jump when /W goes high

    // SM configuration
    sm_reg = PIO1_SM_REG(1);
    sm_reg->clkdiv = PIO_CLKDIV(1, 0);
    sm_reg->execctrl =
        PIO_WRAP_BOTTOM(addr_write_wrap_bottom) |
        PIO_WRAP_TOP(addr_write_wrap_top) |
        PIO_JMP_PIN(12);    // /W pin
    sm_reg->shiftctrl =
        PIO_IN_COUNT(11) |
        //PIO_AUTOPUSH |         // No autopush
        //PIO_PUSH_THRESH(32) |  // No autopush
        PIO_IN_SHIFTDIR_L |
        PIO_OUT_SHIFTDIR_L;
    sm_reg->pinctrl = PIO_IN_BASE(13);  // Address base pin
    PIO1_SM_TXF(1) = ram_table_high_bits;
    sm_reg->instr = PULL_BLOCK;
    sm_reg->instr = MOV_X_OSR;
    sm_reg->instr = JMP(addr_write_start);

    // Now copy all PIO1 instructions
    for (int ii = 0; ii < offset; ii++) {
        PIO1_INSTR_MEM(ii) = instr_scratch[ii];
    }

#if defined(DEBUG_LOGGING)
    pio_log_sm(
        "Address Reader (RAM READ)",
        1,
        0,
        (uint32_t *)instr_scratch,
        addr_wrap_1st_instr,
        addr_wrap_start
    );
    pio_log_sm(
        "Address Reader (RAM WRITE)",
        1,
        1,
        (uint32_t *)instr_scratch,
        addr_write_1st_instr,
        addr_write_start
    );
#endif // DEBUG_LOGGING

    //
    // PIO2 Programs
    //
    // Data Handlers
    //
    offset = 0;

    //
    // PIO2 - Data Handlers
    //
    // SM0 - Data Input/Output handler
    //
    // Start by setting data pins to inputs
    uint8_t data_io_1st_instr = offset;
    uint8_t data_io_write_enabled = offset;
    uint8_t data_io_start = offset;
    instr_scratch[offset++] = MOV_PINDIRS_NULL; // Set data pins to inputs
    // Test for /CE and /OE active
    uint8_t data_io_wrap_bottom = offset;
    instr_scratch[offset++] = MOV_X_PINS;
    instr_scratch[offset++] = JMP_X_DEC(data_io_start);     // /CE or /OE inactive.  Have to jump
                                                            // to start and set pins to inputs cos
                                                            // this part of the loop is also used
                                                            // when pins may already be outputs.

    // /CE and /OE low - both active.  Check /W state next
    uint8_t data_io_set_outputs = offset + 2;               // Point to set data pins as outputs
    instr_scratch[offset++] = JMP_PIN(data_io_set_outputs); // /W disabled, do enable
    instr_scratch[offset++] = JMP(data_io_write_enabled);           // /W enabled, don't enable
    uint8_t data_io_wrap_top = offset;
    instr_scratch[offset++] = MOV_PINDIRS_NOT_NULL;         // Set data pins to outputs

    // Configure SM
    sm_reg = PIO2_SM_REG(0);
    sm_reg->clkdiv = PIO_CLKDIV(1, 0);
    sm_reg->execctrl = 
        PIO_WRAP_BOTTOM(data_io_wrap_bottom) |
        PIO_WRAP_TOP(data_io_wrap_top) |
        PIO_JMP_PIN(12);    // /W pin
    sm_reg->shiftctrl = 
        PIO_IN_COUNT(2) |   // /OE amd /CE
        PIO_IN_SHIFTDIR_L;  // Direction doesn't matter
    sm_reg->pinctrl = 
        PIO_IN_BASE(10) |   // /OE and /CE
        PIO_OUT_COUNT(8) |  // Data pins 
        PIO_OUT_BASE(0);    // Data base pin
    sm_reg->instr = JMP(data_io_start);

    //
    // PIO2 - Data Handlers
    //
    // SM1 - Data output (RAM READ)
    //
    // Just waits until 8 bits are made available by the READ DMA chain, then
    // writes them to the data pin outputs (whether they are set to outputs
    // or not).
    uint8_t data_out_1st_instr = offset;
    uint8_t data_out_start = offset;
    uint8_t data_out_wrap_top = offset;
    uint8_t data_out_wrap_bottom = offset;
    instr_scratch[offset++] = OUT_PINS(8);  // Autopull, blocks until 8 bits available
    sm_reg = PIO2_SM_REG(1);
    sm_reg->clkdiv = PIO_CLKDIV(1, 0);
    sm_reg->execctrl = 
        PIO_WRAP_BOTTOM(data_out_wrap_bottom) |
        PIO_WRAP_TOP(data_out_wrap_top);
    sm_reg->shiftctrl = 
        PIO_OUT_SHIFTDIR_R |    // Writes LSB of OSR
        PIO_AUTOPULL |          // Auto pull when we hit threshold
        PIO_PULL_THRESH(8);     // Pull when we have 8 bits
    sm_reg->pinctrl =
        PIO_OUT_COUNT(8) |      // Number of data lines
        PIO_OUT_BASE(0);        // Data base pin
    sm_reg->instr = JMP(data_out_start);

    //
    // PIO2 - Data Handlers
    //
    // SM2 - Data input (RAM WRITE)
    //
    uint8_t data_in_1st_instr = offset; 
    uint8_t data_in_valid = offset;
    instr_scratch[offset++] = PUSH_BLOCK;               // Push data to RX FIFO for DMA
    uint8_t data_in_start = offset;
    instr_scratch[offset++] = WAIT_IRQ_HIGH_NEXT(3);    // Wait for RAM WRITE IRQ
    uint8_t data_in_wrap_bottom = offset;
    instr_scratch[offset++] = NOP;  // Synchronise with address reader which takes 2 cycles to read
    instr_scratch[offset++] = MOV_ISR_PINS;             // Read at same time as address pins
    uint8_t data_in_wrap_top = offset;
    instr_scratch[offset++] = JMP_PIN(data_in_valid);   // Jump when /W goes high 

    sm_reg = PIO2_SM_REG(2);
    sm_reg->clkdiv = PIO_CLKDIV(1, 0);
    sm_reg->execctrl = 
        PIO_WRAP_BOTTOM(data_in_wrap_bottom) |
        PIO_WRAP_TOP(data_in_wrap_top) |
        PIO_JMP_PIN(12);    // /W pin
    sm_reg->shiftctrl =
        PIO_IN_COUNT(8) |       // Number of data lines
        // PIO_AUTOPUSH |  // No autopush
        // PIO_PUSH_THRESH(8) |    // Number of bits to push (number of data lines)
        PIO_IN_SHIFTDIR_L;
    sm_reg->pinctrl =
        PIO_IN_BASE(0);         // Data base pin
    sm_reg->instr = JMP(data_in_start);

    // Now copy all PIO2 instructions
    for (int ii = 0; ii < offset; ii++) {
        PIO2_INSTR_MEM(ii) = instr_scratch[ii];
    }

#if defined(DEBUG_LOGGING)
    pio_log_sm(
        "Data IO Handler",
        2,
        0,
        (uint32_t *)instr_scratch,
        data_io_1st_instr,
        data_io_start
    );
    pio_log_sm(
        "Data Reader (RAM READ)",
        2,
        1,
        (uint32_t *)instr_scratch,
        data_out_1st_instr,
        data_out_start
    );
    pio_log_sm(
        "Data Reader (RAM WRITE)",
        2,
        2,
        (uint32_t *)instr_scratch,
        data_in_1st_instr,
        data_in_start
    );
#endif // DEBUG_LOGGING
}

// Setup DMA channels for RAM serving
static void pioram_setup_dma(pioram_config_t *config) {
    volatile dma_ch_reg_t *dma_reg;
    
    // RP2350 DMA Notes
    //
    // The RP2350's datasheet uses the terms "triggering" and "pacing" and I
    // found it a bit vague and unintuitive.  Specifically "triggering", in
    // datasheet terms, does NOT necessaarily cause a DMA transfer to occur.
    // The channel has to be "paced" as well.
    // 
    // Hence I've have here described DMA it with the concepts of arming and
    // triggering, with triggering differing slightly from the datasheet's
    // usage.  I find the firearm analogy more useful.
    //
    // There are three ways to _arm_ a DMA channel:
    // - Set the transfer_count to a non-zero value.
    // - Write to a _TRIG register associated with the channel.
    // - Chain to a channel from another channel.
    //
    // You must also enable the channel.
    //
    // In terms of _triggering_ a DMA channel, that is causing it to actually
    // "fire", there are essentially three options:
    // - Have it trigger by a DREQ signal, which is generated by another
    //   peripheral like a PIO RX or TX FIFO.
    // - Have it trigger off a timer.
    // - Have it trigger off an arming event.
    //
    // The datasheet describes the trigger here as "pacing" the DMA.
    //
    // The transfer_count decrements after each "firing" (i.e. trigger event)
    // and the DMA "fires" until the transfer_count reaches zero.
    //
    // Once it resets zero, it can be manually reset (i.e. the value changed),
    // or it can be automatically reset by re-arming it - i.e. one of the
    // re-arming mechanisms described above, or mode 0x1 below.
    //
    // The top nibble of the transfer_count has special meanings on the RP2350:
    // - 0x0 - Normal operation
    // - 0x1 - Immediately resets transfer_count back to its original value
    // - 0xf - Never decrement transfer_count (i.e. infinite)
    //
    // Note that 0xf000_0000 never runs - as while the transfer_count never
    // decrements, it is still initially zero.
    //
    // Chaining to another channel has an interesting configuration mechanism.
    // The channel to be chained to is configured within the CTRL_TRIG
    // register.  A value of this channel disables chaining.  To chain a
    // channel to itself is done via a transfer_count of 0xfxxx_xxxx.
    //
    // There is a gotcha here.  Chaining to channel 0 is configured by a lack
    // of setting the CHAIN_TO bits in CTRL_TRIG.  Hence, failing to set the
    // CHAIN_TO bits means chain to channel 0.  This caused me no end of
    // confusion not understanding why a DMA channel of 0 kept being re-armed.
    // In fact, it was being chained to by channel 1, through a lack of
    // explicitly configuring the CHAIN_TO bits within it.  When I added
    // channels 2&3 for RAM WRITEs, I didn't understand why, instead I needed
    // explicit chaining or a transfer_count of 0xfxxx_xxxx to get the DMA to
    // run forever.  The datasheet does flag this within the CTRL_TRIG
    // description:
    //
    // "Note this field resets to 0, so channels 1 and above will chain to
    // channel 0 bydefault. Set this field to avoid this behaviour."
    //
    // IRQ_QUIET is set to avoid IRQs being raised each time transfer_count
    // reaches zero.  This is not strictly required, as this firmware doesn't
    // service those interrupts, but is cleaner.
    //
    // There is also the concept of DMA priorities - each channel can be normal
    // or high priority.  All high priority channels and a maximum of one low
    // priority channel will be scheduled in each cycle _if there is DMA
    // saturation_.
    //
    // This description ignores the read and write incrementing modes, and ring
    // buffer support, as they are not used by One ROM.

    // RAM Serving DMA Configuration Notes
    //
    // Each of the RAM READ and WRITE operations use a chain of two DMAs.  The
    // first DMA in each chain reads the target RAM address from the appropriate
    // PIO RX FIFO and writes it to the second DMA's ADDR_TRIG register, which
    // arms it.  The second DMA then performs the actual data transfer to or
    // from the RAM table in SRAM, using another PIO FIFO as the source or
    // destination.
    //
    // The first DMA in each chain is triggered by it's address reader PIO
    // pushing an address to its RX FIFO.
    //
    // The READ chain is driven by its PIO continuously - that PIO tends to sit
    // in a tight loop readig and pushing.  That PIO has strategic NOPs to
    // avoid overloading the DMA chain.  It might seem counterintuitive to run
    // this chain continuously, even when data is being written, but as the
    // data lines are only set to outputs when /CE and /OE are both low, it
    // does not interfere with RAM WRITEs, although see the next paragraph.
    //
    // The WRITE chain is configured as high priority, to ensure that, if there
    // is contention, it gets serviced before the READ chain.  Contention is
    // relatively likely, becaus the READ chain runs continuously, even when
    // the RAM is in WRITE mode.  Hence when in WRITE mode, the READ chain's
    // DMA may be trying to access the SRAM at the same time as the WRITE
    // chain.  Setting the WRITE chain to high priority helps ensure that the
    // WRITE gets serviced first, reducing the chance of the READ DMA causing
    // delays to the WRITE operation.

    //
    // READ Chain DMAs
    //
    
    // DMA0 - Address Forwarder (READ)
    dma_reg = DMA_CH_REG(0);
    dma_reg->read_addr = (uint32_t)&PIO1_SM_RXF(0);         // Read from RAM READ address reader RX FIFO
    dma_reg->write_addr = (uint32_t)&DMA_CH_READ_ADDR_TRIG(1);  // Write to DMA1 to re-arm it
    dma_reg->transfer_count = 0xffffffff;                   // Re-arm self
    dma_reg->ctrl_trig =
        DMA_CTRL_TRIG_EN |                                  // Enable DMA
        DMA_CTRL_TRIG_IRQ_QUIET |                           // No IRQs
        DMA_CTRL_TRIG_TREQ_SEL(DREQ_PIO_X_SM_Y_RX(1, 0)) |  // Triggered by RAM READ address reader RX FIFO
        DMA_CTRL_TRIG_DATA_SIZE_32BIT |                     // Read a 32-bit RAM READ target address
        DMA_CTRL_TRIG_CHAIN_TO(0);                          // Disable chaining
    
    // DMA1 - Data Fetcher (READ)
    dma_reg = DMA_CH_REG(1);
    dma_reg->read_addr = config->ram_table_addr;            // Placeholder value, written to by DMA0
    dma_reg->write_addr = (uint32_t)&PIO2_SM_TXF(1);        // Write to RAM READ data writer TX FIFO
    dma_reg->transfer_count = 1;                            // Run once, then require re-arming by DMA0 writing to ADDR_TRIG register
    dma_reg->ctrl_trig =
        DMA_CTRL_TRIG_EN |                                  // Enable DMA
        DMA_CTRL_TRIG_IRQ_QUIET |                           // No IRQs
        DMA_CTRL_TRIG_TREQ_SEL(DMA_CTRL_TRIG_TREQ_PERM) |   // Triggered by arming
        DMA_CTRL_TRIG_DATA_SIZE_8BIT |                      // Write 8-bit RAM READ data
        DMA_CTRL_TRIG_CHAIN_TO(0);                          // Disable chaining
    
    //
    // WRITE Chain DMAs
    //
    
    // DMA2 - Address Forwarder (WRITE)
    dma_reg = DMA_CH_REG(2);
    dma_reg->read_addr = (uint32_t)&PIO1_SM_RXF(1);         // Read from RAM WRITE address reader RX FIFO
    dma_reg->write_addr = (uint32_t)&DMA_CH_WRITE_ADDR_TRIG(3);  // Trigger DMA3 to store the data byte
    dma_reg->transfer_count = 0xffffffff;                   // Re-arm self
    dma_reg->ctrl_trig =
        DMA_CTRL_TRIG_EN |                                  // Enable DMA
        DMA_CTRL_TRIG_IRQ_QUIET |                           // No IRQs
        DMA_CTRL_TRIG_PRIORITY_HIGH |                       // High priority
        DMA_CTRL_TRIG_TREQ_SEL(DREQ_PIO_X_SM_Y_RX(1, 1)) |  // Triggered by RAM WRITE address reader RX FIFO
        DMA_CTRL_TRIG_DATA_SIZE_32BIT |                     // Read a 32-bit RAM WRITE target address
        DMA_CTRL_TRIG_CHAIN_TO(2);                          // Disable chaining
    
    // DMA3 - Data Writer (WRITE)
    dma_reg = DMA_CH_REG(3);
    dma_reg->read_addr = (uint32_t)&PIO2_SM_RXF(2);         // Read from RAM WRITE data reader RX FIFO
    dma_reg->write_addr = config->ram_table_addr;           // Placeholder, gets overwritten by DMA2
    dma_reg->transfer_count = 1;
    dma_reg->ctrl_trig =
        DMA_CTRL_TRIG_EN |                                  // Enable DMA
        DMA_CTRL_TRIG_IRQ_QUIET |                           // No IRQs
        DMA_CTRL_TRIG_PRIORITY_HIGH |                       // High priority
        DMA_CTRL_TRIG_DATA_SIZE_8BIT |                      // Store 8-bit RAM WRITE data
        DMA_CTRL_TRIG_TREQ_SEL(DMA_CTRL_TRIG_TREQ_PERM) |   // Triggered by arming
        DMA_CTRL_TRIG_CHAIN_TO(3);                          // Disable chaining
    
    // Set DMA high priority (over CPU access).  It would be possible 
    BUSCTRL_BUS_PRIORITY |=
        BUSCTRL_BUS_PRIORITY_DMA_R_BIT |
        BUSCTRL_BUS_PRIORITY_DMA_W_BIT;
}

// Set GPIOs to PIO function for RAM serving
static void pioram_set_gpio_func(pioram_config_t *config) {
    (void)config;

    // CS pins - not required, as always inputs, and all PIOs can access inputs
    // all the time
    // GPIO_CTRL(10) = GPIO_CTRL_FUNC_PIO2; // /OE
    // GPIO_CTRL(11) = GPIO_CTRL_FUNC_PIO2; // /CE
    // GPIO_CTRL(12) = GPIO_CTRL_FUNC_PIO2; // /W

    // Address pins - not required, as always inputs
    //for (int ii = 13; ii <= 23; ii++) {
    //    GPIO_CTRL(ii) = GPIO_CTRL_FUNC_PIO1;
    //}

    // Data pins
    for (int ii = 0; ii < 8; ii++) {
        GPIO_CTRL(ii) = GPIO_CTRL_FUNC_PIO2;
    }
}

// Start all PIO state machines
static void pioram_start_pios(void) {
    PIO0_CTRL_SM_ENABLE(0x1);  // Enable SM0
    PIO1_CTRL_SM_ENABLE(0x3);  // Enable SM0 and SM1
    PIO2_CTRL_SM_ENABLE(0x7);  // Enable SM0, SM1, and SM2
    DEBUG("RAM PIOs started");
}

extern uint32_t _ram_rom_image_start[];

// Top-level RAM serving function
void pioram(
    const sdrr_info_t *info,
    uint32_t ram_table_addr
) {
    (void)info;

    DEBUG("%s", log_divider);

    ram_table_addr = (uint32_t)_ram_rom_image_start;

    // Clear 64KB RAM table
    uint8_t *ram_table_ptr = (uint8_t *)ram_table_addr;
    for (int ii = 0; ii < 65536; ii++) {
        ram_table_ptr[ii] = 0x03;
    }

    pioram_config_t config = {
        .read_cs_base_pin = 10,  // /OE + /CE, fire-24-d
        .num_read_cs_pins = 2,
        .write_cs_base_pin = 11, // /CE + /W, fire-24-d
        .num_write_cs_pins = 2,
        .data_base_pin = 0,  // fire-24-d
        .num_data_pins = 8,
        .addr_base_pin = 13,  // fire-24-d
        .num_addr_pins = 11,  // 6116 has A0-A10
        .ram_table_addr = ram_table_addr,
        .read_cs_clkdiv_int = 1,
        .read_cs_clkdiv_frac = 0,
        .write_cs_clkdiv_int = 1,
        .write_cs_clkdiv_frac = 0,
        .addr_clkdiv_int = 1,
        .addr_clkdiv_frac = 0,
        .data_out_clkdiv_int = 1,
        .data_out_clkdiv_frac = 0,
        .data_in_clkdiv_int = 1,
        .data_in_clkdiv_frac = 0,
    };
    
    // Validate configuration
    if (ram_table_addr & 0xFFFF) {
        LOG("!!! PIO RAM serving requires RAM table address to be 64KB aligned");
        limp_mode(LIMP_MODE_INVALID_CONFIG);
    }
    
    // Bring PIO0, PIO1, PIO2 and DMA out of reset
    RESET_RESET &= ~(RESET_PIO0 | RESET_PIO1 | RESET_PIO2 | RESET_DMA);
    while (!(RESET_DONE & (RESET_PIO0 | RESET_PIO1 | RESET_PIO2 | RESET_DMA)));
    
    // Setup DMA channels
    pioram_setup_dma(&config);
    
    // Configure GPIOs
    pioram_set_gpio_func(&config);

    // Load PIO programs
    pioram_load_programs(&config);
    
    // Start PIOs
    pioram_start_pios();
    DEBUG("PIO RAM serving started");
    DEBUG("%s", log_divider);

#define PIO_DEBUG_LOOP 1
#if defined(PIO_DEBUG_LOOP)
    // Output PIO and DMA debug information periodically
    uint32_t last_read_addr = 0xFFFFFFFF;
    uint32_t last_write_addr = 0xFFFFFFFF;
    uint8_t read_addr_still_unchanged = 0;
    uint8_t write_addr_still_unchanged = 0;
    while (1) {
        // See if any PIO FIFOs are full
        uint32_t volatile pio_fstats[3] = {
            PIO0_FSTAT,
            PIO1_FSTAT,
            PIO2_FSTAT
        };
        for (int ii = 0; ii < 3; ii++) {
            uint32_t pio_fstat = pio_fstats[ii];
            for (int jj = 0; jj < 4; jj++) {
                uint8_t rxfull_bit = 0 + jj;
                uint8_t txfull_bit = 16 + jj;
                if (pio_fstat & (1 << rxfull_bit)) {
                    DEBUG("!!! PIO%d SM%d RXFULL set", ii, jj);
                }
                if (pio_fstat & (1 << txfull_bit)) {
                    DEBUG("!!! PIO%d SM%d TXFULL set", ii, jj);
                }
            }
        }

        // Check the DMA read/write RAM table addresses are changing.
        // The odd log here is acceptable - but constant unchanging read or
        // write addresses suggest a problem (for example, host has crashed).
        // As such we only log if at least the last three checks have been
        // the same.
        volatile dma_ch_reg_t *dma1 = DMA_CH_REG(1);
        volatile dma_ch_reg_t *dma3 = DMA_CH_REG(3);
        uint32_t new_read_addr = dma1->read_addr;
        uint32_t new_write_addr = dma3->write_addr;
        if (new_read_addr == last_read_addr) {
            if (read_addr_still_unchanged > 1) {
                DEBUG("!!! RAM READ address unchanged: 0x%08X", new_read_addr);
            }
            read_addr_still_unchanged++;
        } else {
            read_addr_still_unchanged = 0;
        }
        if (new_write_addr == last_write_addr) {
            if (write_addr_still_unchanged > 1) {
                DEBUG("!!! RAM WRITE address unchanged: 0x%08X", new_write_addr);
            }
            write_addr_still_unchanged++;
        } else {
            write_addr_still_unchanged = 0;
        }
        last_read_addr = new_read_addr;
        last_write_addr = new_write_addr;

        // Delay before next check
        #define PIO_DEBUG_LOOP_DELAY 1000000
        for (volatile int i = 0; i < PIO_DEBUG_LOOP_DELAY; i++);
    }
#endif // PIO_DEBUG_LOOP

    // Low power loop
    while (1) {
        __asm volatile("wfi");
    }
}


#endif // RP235X