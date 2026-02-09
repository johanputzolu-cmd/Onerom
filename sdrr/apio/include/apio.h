// Copyright (C) 2026 Piers Finlayson <piers@piers.rocks>
//
// MIT License

// One ROM RP2350 Single-pass Inline PIO Assembler
//
// Provides macros to construct PIO programs for RP2350 PIO state machines.

#ifndef PIOASM_H
#define PIOASM_H

//
// Macros to build PIO programs
//

// Instructions:
//
// You MUST build all SMs for a single PIO block and write them using
// `PIO_END_BLOCK()` before moving onto the next PIO block, as a single
// stack based scratch buffer is used by these macros.
//
// 1.  At the beginning of your PIO building function, call `PIO_ASM_INIT()`
//     to declare and initialise the necessary variables.
//
// 2.  (Optional) Clear all PIO IRQs using `PIO_CLEAR_ALL_IRQS()`.
//
// 3.  Start the first block with `PIO_SET_BLOCK(BLOCK_NUM)` where `BLOCK_NUM`
//     is 0, 1 or 2.
//
// 4.  Starting with SM 0, start the first program with `PIO_SET_SM(SM_NUM)`
//     where `SM_NUM` is 0, 1, 2 or 3.
//
// 5.  (Optional) Create any labels required before the next instruction using
//     `PIO_LABEL_NEW(LABEL_NAME)`, where `LABEL_NAME` must be unique within
//     your function.  These labels are used as destinations for JMP
//     instructions.
//
// 6.  (Optional) Use `PIO_START()`, `PIO_WRAP_BOTTOM()`, `PIO_WRAP_TOP()` and
//     `PIO_END()` before the instruction, to mark the start and wrap points of
//     your program.
// 
//     `PIO_WRAP_TOP()` must be called _before_ adding the instruction that is
//     to be the wrap top.  You do not need to call these macros if the
//     .start, .wrap_bottom or .wrap_top are to be at instruction 0.
//
//     `PIO_END()` is only required if the program ends beyond .wrap, and, if
//     used, must be called _after_ `PIO_WRAP_TOP()`, but before the final
//     instruction of the program.
//
// 7.  Add PIO instructions using `PIO_ADD_INSTR(INSTRUCTION)`.
//
// 8.  Repeat steps 5 to 7 for this SM's program.
//
// 9.  Call 'PIO_SM_CLKDIV_SET(INT, FRAC)` to set the SM's clock divider.
//
// 10. Call `PIO_SM_EXECCTRL_SET(EXECCTRL)` to set the SM's EXECCTRL register.
//     There is no need to encode the wrap top and bottom here, as they are
//     handled automatically.
//
// 11. Call `PIO_SM_SHIFTCTRL_SET(SHIFTCTRL)` to set the SM's SHIFTCTRL
//     register.
//
// 12. Call `PIO_SM_PINCTRL_SET(PINCTRL)` to set the SM's PINCTRL register.
//
// 13. (Optional) Use `PIO_SM_INSTR_SET(INSTRUCTION)` to execute discrete
//     instructions on this SM immediately after configuration.
//
// 14. Call `PIO_SM_JMP_TO_START()` to set the SM to jump to the start of the
//     program after configuration.
//
// 15. (Optional) Call `PIO_LOG_SM("SM NAME")` to log the SM program details
//     for debugging.
//
// 16. (Optional) Repeat steps 4 to 15 for each additional SM in this PIO block.
//
// 17. Call `PIO_END_BLOCK()` to write all constructed programs to the PIO
//     instruction memory.
//
// 18. Repeat steps 3 to 17 for each additional PIO block.

#include <stdint.h>
#include <pioreg.h>
#include <apio_dis.h>

#define MAX_PIO_INSTRS      32
#define MAX_SMS_PER_BLOCK   4
#define MAX_PIO_BLOCKS      3
#define MAX_FIFO_DEPTH      4

#if defined(APIO_EMULATION)
#define MAX_PRE_INSTRS   16
typedef struct {
    uint32_t irq[MAX_PIO_BLOCKS];
    uint8_t first_instr[MAX_PIO_BLOCKS][MAX_SMS_PER_BLOCK];
    uint8_t start[MAX_PIO_BLOCKS][MAX_SMS_PER_BLOCK];
    uint8_t end[MAX_PIO_BLOCKS][MAX_SMS_PER_BLOCK];
    uint8_t wrap_bottom[MAX_PIO_BLOCKS][MAX_SMS_PER_BLOCK];
    uint8_t wrap_top[MAX_PIO_BLOCKS][MAX_SMS_PER_BLOCK];
    pio_sm_reg_t pio_sm_reg[MAX_PIO_BLOCKS][MAX_SMS_PER_BLOCK];
    uint16_t instr[MAX_PIO_BLOCKS][MAX_PIO_INSTRS];
    uint16_t pre_instr[MAX_PIO_BLOCKS][MAX_SMS_PER_BLOCK][MAX_PRE_INSTRS];
    uint8_t pre_instr_count[MAX_PIO_BLOCKS][MAX_SMS_PER_BLOCK];
    uint32_t tx_fifos[MAX_PIO_BLOCKS][MAX_SMS_PER_BLOCK][MAX_FIFO_DEPTH];
    uint8_t tx_fifo_count[MAX_PIO_BLOCKS][MAX_SMS_PER_BLOCK];
    uint32_t rx_fifos[MAX_PIO_BLOCKS][MAX_SMS_PER_BLOCK][MAX_FIFO_DEPTH];
    uint8_t rx_fifo_count[MAX_PIO_BLOCKS][MAX_SMS_PER_BLOCK];
    uint8_t offset[MAX_PIO_BLOCKS];
    uint8_t max_offset[MAX_PIO_BLOCKS];
    uint8_t enabled_sms[MAX_PIO_BLOCKS];
    uint8_t block;
    uint8_t sm;
    uint8_t block_ended[MAX_PIO_BLOCKS];
    uint8_t pios_enabled;
    uint32_t gpio_base[MAX_PIO_BLOCKS];
} test_pio_t;
extern test_pio_t test_pio;
#define __blk  test_pio.block
#define __sm   test_pio.sm
#define __pio_start   test_pio.start
#define __pio_wrap_bottom   test_pio.wrap_bottom
#define __pio_wrap_top   test_pio.wrap_top
#define __pio_end   test_pio.end
#define __pio_offset test_pio.offset
#define __pio_first_instr test_pio.first_instr
#undef PIO0_SM_REG
#define PIO0_SM_REG(SM)  (&test_pio.pio_sm_reg[__blk][SM])
#undef PIO1_SM_REG
#define PIO1_SM_REG(SM)  (&test_pio.pio_sm_reg[__blk][SM])
#undef PIO2_SM_REG
#define PIO2_SM_REG(SM)  (&test_pio.pio_sm_reg[__blk][SM])
#if defined(TEST_PIO_C)
test_pio_t test_pio = {
    .irq = {0xFFFFFFFF},
    .first_instr = {{0xFF}},
    .start = {{0xFF}},
    .end = {{0xFF}},
    .wrap_bottom = {{0xFF}},
    .wrap_top = {{0xFF}},
    .pio_sm_reg = {{{
        .clkdiv = 0xFFFFFFFF,
        .execctrl = 0xFFFFFFFF,
        .shiftctrl = 0xFFFFFFFF,
        .pinctrl = 0xFFFFFFFF
    }}},
    .instr = {{0xFFFF}},
    .pre_instr = {{{0xFFFF}}},
    .pre_instr_count = {{0xFF}},
    .tx_fifos = {{{0xFFFFFFFF}}},
    .tx_fifo_count = {{0xFF}},
    .rx_fifos = {{{0xFFFFFFFF}}},
    .rx_fifo_count = {{0xFF}},
    .offset = {0xFF},
    .max_offset = {0xFF},
    .enabled_sms = {0xFF},
    .block = 0xFF,
    .sm = 0xFF,
    .block_ended = {0xFF},
    .pios_enabled = 0xFF
};
#endif // TEST_PIO_C
#endif // APIO_EMULATION

// Internal macros - do not use directly
#define STATIC_BLOCK_ASSERT(BLOCK)  _Static_assert((BLOCK) >= 0 && (BLOCK) < MAX_PIO_BLOCKS, "Invalid PIO block")

// Internal macro - do not use directly
#define STATIC_SM_ASSERT(SM)        _Static_assert((SM) >= 0 && (SM) < MAX_SMS_PER_BLOCK, "Invalid PIO state machine")

// Internal macro - do not use directly
#define OFFSET_ARRAY_INIT       {{0, 0, 0, 0}, {0, 0, 0, 0}, {0, 0, 0, 0}}

#if !defined(APIO_EMULATION)
#define IF_APIO_EMULATION(CODE)
#else // APIO_EMULATION
#define IF_APIO_EMULATION(CODE) CODE
#endif // !APIO_EMULATION

// Clears IRQs for the specified PIO block
#if !defined(APIO_EMULATION)
#define PIO_CLEAR_IRQ(BLOCK)    STATIC_BLOCK_ASSERT(BLOCK); \
                                if (BLOCK == 0) {           \
                                    PIO0_IRQ = 0xFFFFFFFF;  \
                                } else if (BLOCK == 1) {    \
                                    PIO1_IRQ = 0xFFFFFFFF;  \
                                } else {                    \
                                    PIO2_IRQ = 0xFFFFFFFF;  \
                                }
#else // APIO_EMULATION


#endif // !APIO_EMULATION

// Clear all PIO IRQs
#if !defined(APIO_EMULATION)
#define PIO_CLEAR_ALL_IRQS()    PIO0_IRQ = 0xFFFFFFFF;  \
                                PIO1_IRQ = 0xFFFFFFFF;  \
                                PIO2_IRQ = 0xFFFFFFFF
#else // APIO_EMULATION
#define PIO_CLEAR_ALL_IRQS()    for (int __i = 0; __i < MAX_PIO_BLOCKS; __i++) { \
                                    test_pio.irq[__i] = 0xFFFFFFFF; \
                                }
#endif // !APIO_EMULATION

// Call before creating PIO programs
//
// Uses around 128 bytes of stack space.
#if !defined(APIO_EMULATION)
#define PIO_ASM_INIT()  \
    uint16_t instr_scratch[MAX_PIO_INSTRS];                                             \
    uint8_t __attribute__((unused)) __pio_first_instr[MAX_PIO_BLOCKS][MAX_SMS_PER_BLOCK] = OFFSET_ARRAY_INIT;    \
    uint8_t __pio_start[MAX_PIO_BLOCKS][MAX_SMS_PER_BLOCK] = OFFSET_ARRAY_INIT;         \
    uint8_t __pio_wrap_bottom[MAX_PIO_BLOCKS][MAX_SMS_PER_BLOCK] = OFFSET_ARRAY_INIT;   \
    uint8_t __pio_wrap_top[MAX_PIO_BLOCKS][MAX_SMS_PER_BLOCK] = OFFSET_ARRAY_INIT;      \
    uint8_t __attribute__((unused)) __pio_end[MAX_PIO_BLOCKS][MAX_SMS_PER_BLOCK] = OFFSET_ARRAY_INIT;   \
    uint8_t __pio_offset[MAX_PIO_BLOCKS] = {0, 0, 0};                                   \
    uint8_t __blk = 0;                                                                \
    uint8_t __sm = 0
#else // APIO_EMULATION
#define PIO_ASM_INIT()  uint8_t _pios_enabled = test_pio.pios_enabled; \
                        test_pio = (test_pio_t){0};                    \
                        test_pio.pios_enabled = _pios_enabled
#endif // !APIO_EMULATION

// Assert these, as if they change, the above stack space calculation must be updated.
_Static_assert((MAX_PIO_BLOCKS == 3), "MAX_PIO_BLOCKS must be 3");
_Static_assert((MAX_SMS_PER_BLOCK == 4), "MAX_SMS_PER_BLOCK must be 4");
_Static_assert((MAX_PIO_INSTRS == 32), "MAX_PIO_INSTRS must be 32");

// Bring PIO blocks out of reset
#if !defined(APIO_EMULATION)
#define PIO_ENABLE()    RESET_RESET &= ~(RESET_PIO0 | RESET_PIO1 | RESET_PIO2 );        \
                        while (!(RESET_DONE & (RESET_PIO0 | RESET_PIO1 | RESET_PIO2)));
#else // APIO_EMULATION
#define PIO_ENABLE()    test_pio.pios_enabled = 1
#endif // !APIO_EMULATION

#if !defined(APIO_EMULATION)
#define PIO_ENABLE_SMS(BLOCK, SMS_MASK)  \
                                STATIC_BLOCK_ASSERT(BLOCK); \
                                _Static_assert((SMS_MASK) > 0 && (SMS_MASK) < (1 << MAX_SMS_PER_BLOCK), "Invalid SMS_MASK"); \
                                if (BLOCK == 0) {           \
                                    PIO0_CTRL_SM_ENABLE(SMS_MASK);  \
                                } else if (BLOCK == 1) {    \
                                    PIO1_CTRL_SM_ENABLE(SMS_MASK);  \
                                } else {                    \
                                    PIO2_CTRL_SM_ENABLE(SMS_MASK);  \
                                }
#else // APIO_EMULATION
#define PIO_ENABLE_SMS(BLOCK, SMS_MASK)  test_pio.enabled_sms[BLOCK] = SMS_MASK
#endif // !APIO_EMULATION

// Set the current PIO block
#define PIO_SET_BLOCK(BLOCK)    STATIC_BLOCK_ASSERT(BLOCK); \
                                __blk = BLOCK

// Set the current PIO SM
#define PIO_SET_SM(SM)          STATIC_SM_ASSERT(SM);                                   \
                                __sm = SM;                                              \
                                __pio_first_instr[__blk][__sm] = __pio_offset[__blk];   \
                                __pio_start[__blk][__sm] = __pio_offset[__blk];         \
                                __pio_wrap_bottom[__blk][__sm] = __pio_offset[__blk];   \
                                __pio_wrap_top[__blk][__sm] = __pio_offset[__blk];      \
                                __pio_end[__blk][__sm] = __pio_offset[__blk]

// Use a label as a destination for JMPs
#define PIO_LABEL(NAME)         __pio_label__##NAME

// Create a label for JMPs
#define PIO_LABEL_NEW(NAME)     uint8_t PIO_LABEL(NAME) = __pio_offset[__blk]

// Create a label for JMPs at a relative offset
#define PIO_LABEL_NEW_OFFSET(NAME, OFFSET)  uint8_t PIO_LABEL(NAME) = __pio_offset[__blk] + (OFFSET)

// Set the start offset within a PIO program - call before `PIO_ADD_INSTR()`
// for the start instruction.
#define PIO_START()             __pio_start[__blk][__sm] = __pio_offset[__blk]

// Get a label representing the start of the current PIO program
#define PIO_START_LABEL()       __pio_start[__blk][__sm]

// Set the end offset within a PIO program - call before `PIO_ADD_INSTR()`
// for the last instruction.  Must be called after `PIO_WRAP_TOP()`.  If
// .wrap is the last instruction, this is not required.
#define PIO_END()               __pio_end[__blk][__sm] = __pio_offset[__blk]

// Set the wrap bottom offset within a PIO program - call before
// `PIO_ADD_INSTR()` for the .wrap_target instruction.
#define PIO_WRAP_BOTTOM()       __pio_wrap_bottom[__blk][__sm] = __pio_offset[__blk]

// Set the wrap top offset within a PIO program - call before
// `PIO_ADD_INSTR()` for the .wrap instruction.
#define PIO_WRAP_TOP()          __pio_wrap_top[__blk][__sm] = __pio_offset[__blk];  \
                                PIO_END()

// Add an instruction to the current PIO program.
#if !defined(APIO_EMULATION)
#if defined(DEBUG_LOGGING) && (DEBUG_LOGGING == 1)
#define PIO_ADD_INSTR(INST)     if (__pio_offset[__blk] >= MAX_PIO_INSTRS) {      \
                                    ERR("PIO program overflow in PIO block %d SM %d", __blk, __sm);   \
                                    limp_mode(LIMP_MODE_INVALID_CONFIG);            \
                                } else {                                            \
                                    instr_scratch[__pio_offset[__blk]++] = INST;  \
                                }
#else // !DEBUG_LOGGING
#define PIO_ADD_INSTR(INST)     instr_scratch[__pio_offset[__blk]++] = INST
#endif // DEBUG_LOGGING
#else // APIO_EMULATION
#define PIO_ADD_INSTR(INST)     if (test_pio.offset[__blk] >= MAX_PIO_INSTRS) {      \
                                    STUB_LOG("PIO program overflow in PIO block %d SM %d", __blk, __sm);   \
                                } else {                                            \
                                    test_pio.instr[__blk][test_pio.offset[__blk]++] = INST;  \
                                }
#endif // !APIO_EMULATION

// Set the clock divider for the current PIO SM.
#define PIO_SM_CLKDIV_SET(INT, FRAC)    pio_sm_reg_ptr(__blk, __sm)->clkdiv = PIO_CLKDIV((INT), (FRAC))

// Set the EXECCTRL for the current PIO SM.  Do not include wrap top/bottom.
// Those will be set automatically from the wrap values.
#define PIO_SM_EXECCTRL_SET(EXECCTRL)   pio_sm_reg_ptr(__blk, __sm)->execctrl =                       \
                                            (EXECCTRL) |                                                \
                                            PIO_WRAP_BOTTOM_AS_REG(__pio_wrap_bottom[__blk][__sm]) |  \
                                            PIO_WRAP_TOP_AS_REG(__pio_wrap_top[__blk][__sm])

// Set the SHIFTCTRL for the current PIO SM.
#define PIO_SM_SHIFTCTRL_SET(SHIFTCTRL) pio_sm_reg_ptr(__blk, __sm)->shiftctrl = (SHIFTCTRL)

// Set the PINCTRL for the current PIO SM.
#define PIO_SM_PINCTRL_SET(PINCTRL)     pio_sm_reg_ptr(__blk, __sm)->pinctrl = (PINCTRL)

static inline volatile pio_sm_reg_t* pio_sm_reg_ptr(uint8_t block, uint8_t sm) {
    if (block == 0) return PIO0_SM_REG(sm);
    else if (block == 1) return PIO1_SM_REG(sm);
    else return PIO2_SM_REG(sm);
}

// Immediately execute an instruction on the current PIO SM.  Can be called
// before enabling the SM to set initial state.
#if !defined(APIO_EMULATION)
#define PIO_SM_EXEC_INSTR(INSTR) pio_sm_reg_ptr(__blk, __sm)->instr = INSTR
#else // APIO_EMULATION
#define PIO_SM_EXEC_INSTR(INSTR) test_pio.pre_instr[__blk][__sm][test_pio.pre_instr_count[__blk][__sm]++] = INSTR;
#endif // !APIO_EMULATION

#if !defined(APIO_EMULATION)
static inline volatile uint32_t* pio_txf_ptr(uint8_t block, uint8_t sm) {
    if (block == 0) return (volatile uint32_t *)(PIO0_BASE + PIO_TXF_OFFSET + (sm * 0x04));
    else if (block == 1) return (volatile uint32_t *)(PIO1_BASE + PIO_TXF_OFFSET + (sm * 0x04));
    else return (volatile uint32_t *)(PIO2_BASE + PIO_TXF_OFFSET + (sm * 0x04));
}
static inline volatile uint32_t* pio_rxf_ptr(uint8_t block, uint8_t sm) {
    if (block == 0) return (volatile uint32_t *)(PIO0_BASE + PIO_RXF_OFFSET + (sm * 0x04));
    else if (block == 1) return (volatile uint32_t *)(PIO1_BASE + PIO_RXF_OFFSET + (sm * 0x04));
    else return (volatile uint32_t *)(PIO2_BASE + PIO_RXF_OFFSET + (sm * 0x04));
}
#endif // !APIO_EMULATION

// Access the current SM's TX FIFO
#if !defined(APIO_EMULATION)
#define PIO_TXF (*pio_txf_ptr(__blk, __sm))
#else // APIO_EMULATION
#define PIO_TXF test_pio.tx_fifos[__blk][__sm][test_pio.tx_fifo_count[__blk][__sm]++]
#endif // !APIO_EMULATION

// Access the current SM's RX FIFO
#if !defined(APIO_EMULATION)
#define PIO_RXF (*pio_rxf_ptr(__blk, __sm))
#else // APIO_EMULATION
#define PIO_RXF test_pio.rx_fifos[__blk][__sm][test_pio.rx_fifo_count[__blk][__sm]++]
#endif // !APIO_EMULATION

// Set the current PIO SM to jump to its start instruction after
// configuration.  The PIO SM will only be started by explicitly enabling.
// This sets the point at which it will start.
#define PIO_SM_JMP_TO_START()   PIO_SM_EXEC_INSTR(JMP(__pio_start[__blk][__sm]))

#if !defined(APIO_EMULATION)
static inline volatile uint32_t* pio_instr_mem_ptr(uint8_t block) {
    if (block == 0) return (volatile uint32_t *)(PIO0_BASE + PIO_INSTR_MEM_OFFSET);
    else if (block == 1) return (volatile uint32_t *)(PIO1_BASE + PIO_INSTR_MEM_OFFSET);
    else return (volatile uint32_t *)(PIO2_BASE + PIO_INSTR_MEM_OFFSET);
}
#endif // !APIO_EMULATION

// Write the constructed PIO programs to the PIO instruction memory for the
// current PIO block.  Call after all SMs for this block have been built,
// before enabling.
#if !defined(APIO_EMULATION)
#define PIO_END_BLOCK() do { \
                            volatile uint32_t* ptr = pio_instr_mem_ptr(__blk);    \
                            for (int ii = 0; ii < __pio_offset[__blk]; ii++) {    \
                                 ptr[ii] = instr_scratch[ii];                        \
                            }                                                       \
                        } while(0)
#else // APIO_EMULATION
#define PIO_END_BLOCK() test_pio.max_offset[__blk] = __pio_offset[__blk]
#endif // !APIO_EMULATION

// Call for each SM to log its information for debugging (`DEBUG_LOGGING`
// must be defined).
#if !defined(APIO_EMULATION)
#if defined(DEBUG_LOGGING)
#define PIO_LOG_SM(NAME)                    \
    apio_log_sm(                            \
        NAME,                               \
        __blk,                              \
        __sm,                               \
        instr_scratch,                      \
        __pio_first_instr[__blk][__sm],     \
        __pio_start[__blk][__sm],           \
        __pio_end[__blk][__sm]              \
    )
#else
#define PIO_LOG_SM(NAME)
#endif // defined(DEBUG_LOGGING)
#else // APIO_EMULATION
#define PIO_LOG_SM(NAME) \
    apio_log_sm(                            \
        NAME,                               \
        __blk,                              \
        __sm,                               \
        test_pio.instr[__blk],              \
        test_pio.first_instr[__blk][__sm],  \
        __pio_start[__blk][__sm],           \
        __pio_end[__blk][__sm]              \
    )
#endif // !APIO_EMULATION

// Call to enable one or more SMs within a PIO block.  To enable more than SM
// simultaneously, OR the SM numbers together (e.g. to enable SM0 and SM2, use
// 0b00000101 = 5).
#if !defined(APIO_EMULATION)
#define PIO_ENABLE_SM(BLOCK, SM_MASK)   STATIC_BLOCK_ASSERT(BLOCK);         \
                                        _Static_assert((SM_MASK < 0xF), "Attempt to enable invalid SM"); \
                                        if (BLOCK == 0) {                   \
                                            PIO0_CTRL_SM_ENABLE(SM_MASK);   \
                                        } else if (BLOCK == 1) {            \
                                            PIO1_CTRL_SM_ENABLE(SM_MASK);   \
                                        } else {                            \
                                            PIO2_CTRL_SM_ENABLE(SM_MASK);   \
                                        }
#else // APIO_EMULATION
#define PIO_ENABLE_SM(BLOCK, SM_MASK)   STATIC_BLOCK_ASSERT(BLOCK);         \
                                        _Static_assert((SM_MASK < 0xF), "Attempt to enable invalid SM"); \
                                        test_pio.enabled_sms[BLOCK] |= SM_MASK
#endif // !APIO_EMULATION

//
// PIO Instruction Macros
//

// Add a side set delay from 0-31 cycles to an instruction
#define ADD_DELAY(INST, DELAY)  ((INST) | (((DELAY) & 0x1F) << 8))

// Move the pins value to the ISR
#define IN_PINS(NUM)            (0x4000 | ((NUM) & 0x1F))

// Move X to the ISR
#define IN_X(NUM)               (0x4020 | ((NUM) & 0x1F))

// Move Y to the ISR
#define IN_Y(NUM)               (0x4040 | ((NUM) & 0x1F))

// Clear one of this PIO block's IRQs
#define IRQ_CLEAR(X)            (0xC040 | ((X) & 0x07))

// Clear one of the previous PIO block's IRQs
#define IRQ_CLEAR_PREV(X)       (0xC048 | ((X) & 0x07))

// Clear one of the next PIO block's IRQs
#define IRQ_CLEAR_NEXT(X)       (0xC058 | ((X) & 0x07))

// Set one of this PIO block's IRQs to 1
#define IRQ_SET(X)              (0xC000 | ((X) & 0x07))

// Set one of the previous PIO block's IRQs to 1
#define IRQ_SET_PREV(X)         (0xC008 | ((X) & 0x07))

// Set one of the next PIO block's IRQs to 1
#define IRQ_SET_NEXT(X)         (0xC018 | ((X) & 0x07))

// Jump unconditionally to label X within this PIO program 
#define JMP(X)                  (0x0000 | ((X) & 0x1F))

// Jump to label if X register is zero
#define JMP_NOT_X(DEST)         (0x0020 | ((DEST) & 0x1F))

// Jump to label if X register is non-zero and then decrement Y after the test
#define JMP_X_DEC(DEST)         (0x0040 | ((DEST) & 0x1F))

// Jump to label if Y register is non-zero and then decrement X after the test
#define JMP_Y_DEC(DEST)         (0x0080 | ((DEST) & 0x1F))

// Jump to label if X register is not equal to Y register
#define JMP_X_NOT_Y(DEST)       (0x00A0 | ((DEST) & 0x1F))

// Jump to label if pin specified as the EXECCTRL JMP_PIN is high
#define JMP_PIN(X)              (0x00C0 | ((X) & 0x1F))

// Set the output pin values to 0 (low)
#define MOV_PINS_NULL           0xA003

// Move the pin values to the X register
#define MOV_X_PINS              0xA020

// Move the OSR into the X register
#define MOV_X_OSR               0xA027

// Set the output pin directions to 0 (inputs)
#define MOV_PINDIRS_NULL        0xA063

// Set the output pin directions to 1 (outputs)
#define MOV_PINDIRS_NOT_NULL    0xA06B

// Move the pin values to the ISR
#define MOV_ISR_PINS            0xA0C0

// No operation (move Y to Y)
#define NOP                     0xA042

// Move data from OSR to the output pins
#define OUT_PINS(NUM)           (0x6000 | ((NUM) & 0x1F))

// Pull data from the TX FIFO into the OSR, blocking if FIFO is empty
#define PULL_BLOCK              0x80A0

// Push data from the ISR into the RX FIFO, blocking if FIFO is full
#define PUSH_BLOCK              0x8020

// Set X register to VALUE (0-31)
#define SET_X(VALUE)            (0xE020 | ((VALUE) & 0x1F))

// Set Y register to VALUE (0-31)
#define SET_Y(VALUE)            (0xE040 | ((VALUE) & 0x1F))

// Wait for one of this PIO block's IRQs to go high.  Clears the IRQ
// after the instruction (so other PIOs waiting at the same time will also be
// triggered).
#define WAIT_IRQ_HIGH(X)        (0x20C0 | ((X) & 0x07))

// Wait for one of the previous PIO block's IRQs to go high.
#define WAIT_IRQ_HIGH_PREV(X)   (0x20C8 | ((X) & 0x07))

// Wait for one of the next PIO block's IRQs to go high.
#define WAIT_IRQ_HIGH_NEXT(X)   (0x20D8 | ((X) & 0x07))

// Wait for one of this PIO block's IRQs to go low.
#define WAIT_IRQ_LOW(X)         (0x2040 | ((X) & 0x07))

// Wait for one of the previous PIO block's IRQs to go low.
#define WAIT_IRQ_LOW_PREV(X)    (0x2048 | ((X) & 0x07))

// Wait for one of the next PIO block's IRQs to go low.
#define WAIT_IRQ_LOW_NEXT(X)    (0x2058 | ((X) & 0x07))

// Wait for the specified pin to go high
#define WAIT_PIN_HIGH(X)        (0x20A0 | ((X) & 0x1F))

#if !defined(APIO_EMULATION)
#define ASM_WFI()               __asm volatile("wfi")
#else // APIO_EMULATION
#define ASM_WFI()               STUB_LOG("WFI called"); return
#endif // !APIO_EMULATION

#endif // PIOASM_H