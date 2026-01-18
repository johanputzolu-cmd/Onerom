// Copyright (C) 2026 Piers Finlayson <piers@piers.rocks>
//
// MIT License

// RP2350 PIO Assembler macros

#ifndef PIOASM_H
#define PIOASM_H

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

//
// Macros to build PIO programs
//

// Instructions:
//
// You MUST build all SMs for a single PIO block and write them using
// `PIO_WRITE_BLOCK()` before moving onto the next PIO block, as a single
// stack based scratch buffer is used by these macros.
//
// 1.  At the beginning of your PIO building function, call `PIO_BUILD_INIT()`
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
// 13. Call `PIO_SM_COMMIT_REGS()` to commit the SM's register values.
//
// 14. (Optional) Use `PIO_SM_INSTR_SET(INSTRUCTION)` to execute discrete
//     instructions on this SM immediately after configuration.
//
// 15. Call `PIO_SM_JMP_TO_START()` to set the SM to jump to the start of the
//     program after configuration.
//
// 16. (Optional) Call `PIO_LOG_SM("SM NAME")` to log the SM program details
//     for debugging.
//
// 17. (Optional) Repeat steps 4 to 16 for each additional SM in this PIO block.
//
// 18. Call `PIO_WRITE_BLOCK()` to write all constructed programs to the PIO
//     instruction memory.
//
// 19. Repeat steps 3 to 18 for each additional PIO block.

// Internal macros - do not use directly
#define STATIC_BLOCK_ASSERT(BLOCK)  _Static_assert((BLOCK) >= 0 && (BLOCK) <=2, "Invalid PIO block")

// Internal macro - do not use directly
#define STATIC_SM_ASSERT(SM)        _Static_assert((SM) >= 0 && (SM) <=3, "Invalid PIO state machine")

// Internal macro - do not use directly
#define OFFSET_ARRAY_INIT       {{0, 0, 0, 0}, {0, 0, 0, 0}, {0, 0, 0, 0}}

// Clears IRQs for the specified PIO block
#define PIO_CLEAR_IRQ(BLOCK)    STATIC_BLOCK_ASSERT(BLOCK); \
                                if (BLOCK == 0) {           \
                                    PIO0_IRQ = 0xFFFFFFFF;  \
                                } else if (BLOCK == 1) {    \
                                    PIO1_IRQ = 0xFFFFFFFF;  \
                                } else {                    \
                                    PIO2_IRQ = 0xFFFFFFFF;  \
                                }

// Clear all PIO IRQs
#define PIO_CLEAR_ALL_IRQS()    PIO0_IRQ = 0xFFFFFFFF;  \
                                PIO1_IRQ = 0xFFFFFFFF;  \
                                PIO2_IRQ = 0xFFFFFFFF;

// Call before creating PIO programs
//
// Uses around 512 bytes of stack space.
#define PIO_BUILD_INIT()    uint32_t instr_scratch[32]; \
                            uint8_t __block;            \
                            uint8_t __sm;               \
                            uint8_t __pio_first_instr[3][4] = OFFSET_ARRAY_INIT; \
                            uint8_t __pio_start[3][4] = OFFSET_ARRAY_INIT;       \
                            uint8_t __pio_wrap_bottom[3][4] = OFFSET_ARRAY_INIT; \
                            uint8_t __pio_wrap_top[3][4] = OFFSET_ARRAY_INIT;    \
                            uint8_t __pio_end[3][4] = OFFSET_ARRAY_INIT;         \
                            uint32_t __pio_clkdiv[3][4] = OFFSET_ARRAY_INIT;     \
                            uint32_t __pio_execctrl[3][4] = OFFSET_ARRAY_INIT;   \
                            uint32_t __pio_shiftctrl[3][4] = OFFSET_ARRAY_INIT;  \
                            uint32_t __pio_pinctrl[3][4] = OFFSET_ARRAY_INIT;    \
                            volatile pio_sm_reg_t *__pio_sm_reg[3][4] = {               \
                                {PIO0_SM_REG(0), PIO0_SM_REG(1), PIO0_SM_REG(2), PIO0_SM_REG(3)}, \
                                {PIO1_SM_REG(0), PIO1_SM_REG(1), PIO1_SM_REG(2), PIO1_SM_REG(3)}, \
                                {PIO2_SM_REG(0), PIO2_SM_REG(1), PIO2_SM_REG(2), PIO2_SM_REG(3)}  \
                            };                                                          \
                            volatile uint32_t* __pio_instr_mem[3] = {                   \
                                (volatile uint32_t *)(PIO0_BASE + PIO_INSTR_MEM_OFFSET),\
                                 (volatile uint32_t *)(PIO1_BASE + PIO_INSTR_MEM_OFFSET),\
                                 (volatile uint32_t *)(PIO2_BASE + PIO_INSTR_MEM_OFFSET) \
                            };                                                          \
                            static uint8_t __pio_offset[3] = {0, 0, 0}

// Set the current PIO block
#define PIO_SET_BLOCK(BLOCK)    STATIC_BLOCK_ASSERT(BLOCK); \
                                __block = BLOCK
// Set the current PIO SM
#define PIO_SET_SM(SM)          STATIC_SM_ASSERT(SM);       \
                                __sm = SM

// Create a label for JMPs
#define PIO_LABEL_NEW(NAME)     uint8_t label__##NAME = __pio_offset[__block];

// Use a label as a destination for JMPs
#define PIO_LABEL(NAME)         label__##NAME

// Set the start offset within a PIO program - call before `PIO_ADD_INSTR()`
// for the start instruction.
#define PIO_START()             __pio_start[__block][__sm] = __pio_offset[__block]

// Set the end offset within a PIO program - call before `PIO_ADD_INSTR()`
// for the last instruction.  Must be called after `PIO_WRAP_TOP()`.  If
// .wrap is the last instruction, this is not required.
#define PIO_END()               __pio_end[__block][__sm] = __pio_offset[__block]

// Set the wrap bottom offset within a PIO program - call before
// `PIO_ADD_INSTR()` for the .wrap_target instruction.
#define PIO_WRAP_BOTTOM()       __pio_wrap_bottom[__block][__sm] = __pio_offset[__block]

// Set the wrap top offset within a PIO program - call before
// `PIO_ADD_INSTR()` for the .wrap instruction.
#define PIO_WRAP_TOP()          __pio_wrap_top[__block][__sm] = __pio_offset[__block];  \
                                __pio_end[__block][__sm] = __pio_offset[__block]

// Add an instruction to the current PIO program.
#define PIO_ADD_INSTR(INST)      instr_scratch[__pio_offset[__block]++] = INST

// Set the clock divider for the current PIO SM.
#define PIO_SM_CLKDIV_SET(INT, FRAC)     __pio_clkdiv[__block][__sm] = PIO_CLKDIV((INT), (FRAC))

// Set the EXECCTRL for the current PIO SM.  Do not include wrap top/bottom.
// That will be set automatically by `PIO_SM_COMMIT_REGS()`.
#define PIO_SM_EXECCTRL_SET(EXECCTRL)    __pio_execctrl[__block][__sm] = (EXECCTRL)

// Set the SHIFTCTRL for the current PIO SM.
#define PIO_SM_SHIFTCTRL_SET(SHIFTCTRL)  __pio_shiftctrl[__block][__sm] = (SHIFTCTRL)

// Set the PINCTRL for the current PIO SM.
#define PIO_SM_PINCTRL_SET(PINCTRL)      __pio_pinctrl[__block][__sm]    = (PINCTRL)

// Immediately execute an instruction on the current PIO SM.  Can be called
// before enabling the SM to set initial state.
#define PIO_SM_INSTR_SET(INSTR)          __pio_sm_reg[__block][__sm]->instr = INSTR

// Commit the current PIO SM's register values.  Call before enabling the SM 
// and before logging it.
#define PIO_SM_COMMIT_REGS()    __pio_sm_reg[__block][__sm]->clkdiv = __pio_clkdiv[__block][__sm];  \
                                __pio_sm_reg[__block][__sm]->execctrl =                             \
                                    __pio_execctrl[__block][__sm] |                                 \
                                    PIO_WRAP_BOTTOM_AS_REG(__pio_wrap_bottom[__block][__sm]) |      \
                                    PIO_WRAP_TOP_AS_REG(__pio_wrap_top[__block][__sm]);             \
                                __pio_sm_reg[__block][__sm]->shiftctrl = __pio_shiftctrl[__block][__sm]; \
                                __pio_sm_reg[__block][__sm]->pinctrl = __pio_pinctrl[__block][__sm]

// Set the current PIO SM to jump to its start instruction after
// configuration.  The PIO SM will only be started by explicitly enabling.
// This sets the point at which it will start.
#define PIO_SM_JMP_TO_START()   PIO_SM_INSTR_SET(JMP(__pio_start[__block][__sm]))

// Write the constructed PIO programs to the PIO instruction memory for the
// current PIO block.  Call after all SMs for this block have been built,
// before enabling.
#define PIO_WRITE_BLOCK()       for (int ii = 0; ii < __pio_offset[__block]; ii++) {       \
                                    __pio_instr_mem[__block][ii] = instr_scratch[ii];        \
                                }

// Call for each SM to log its information for debugging (`DEBUG_LOGGING`
// must be defined).
#if defined(DEBUG_LOGGING)
#define PIO_LOG_SM(NAME)                   \
    pio_log_sm(                            \
        NAME,                              \
        __block,                           \
        __sm,                              \
        (uint32_t *)instr_scratch,         \
        __pio_first_instr[__block][__sm],  \
        __pio_start[__block][__sm],        \
        __pio_end[__block][__sm]           \
    )
#else
#define PIO_LOG_SM(NAME)
#endif // defined(DEBUG_LOGGING)

// Call to enable one or more SMs within a PIO block.  To enable more than SM
// simultaneously, OR the SM numbers together (e.g. to enable SM0 and SM2, use
// 0b00000101 = 5).
#define PIO_ENABLE_SM(BLOCK, SM_MASK)   STATIC_BLOCK_ASSERT(BLOCK);         \
                                        _Static_assert((SM_MASK < 0xF), "Attempt to enable invalid SM"); \
                                        if (BLOCK == 0) {                   \
                                            PIO0_CTRL_SM_ENABLE(SM_MASK);   \
                                        } else if (BLOCK == 1) {            \
                                            PIO1_CTRL_SM_ENABLE(SM_MASK);   \
                                        } else {                            \
                                            PIO2_CTRL_SM_ENABLE(SM_MASK);   \
                                        }

#endif // PIOASM_H