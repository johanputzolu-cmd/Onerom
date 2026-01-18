// Copyright (C) 2026 Piers Finlayson <piers@piers.rocks>
//
// MIT License

// RP2350 PIO Assembler macros

#ifndef PIOASM_H
#define PIOASM_H

// Add a side set delay to an instruction
#define ADD_DELAY(INST, DELAY)  ((INST) | (((DELAY) & 0x1F) << 8))

#define IN_PINS(NUM)            (0x4000 | ((NUM) & 0x1F))
#define IN_X(NUM)               (0x4020 | ((NUM) & 0x1F))
#define IN_Y(NUM)               (0x4040 | ((NUM) & 0x1F))

#define IRQ_CLEAR(X)            (0xC040 | ((X) & 0x07))
#define IRQ_CLEAR_PREV(X)       (0xC058 | ((X) & 0x07))
#define IRQ_CLEAR_NEXT(X)       (0xC048 | ((X) & 0x07))
#define IRQ_SET(X)              (0xc000 | ((X) & 0x07))
#define IRQ_SET_PREV(X)         (0xC008 | ((X) & 0x07))
#define IRQ_SET_NEXT(X)         (0xC018 | ((X) & 0x07))

#define JMP(X)                  (0x0000 | ((X) & 0x1F))
#define JMP_NOT_X(DEST)         (0x0020 | ((DEST) & 0x1F))
#define JMP_X_DEC(DEST)         (0x0040 | ((DEST) & 0x1F))
#define JMP_Y_DEC(DEST)         (0x0080 | ((DEST) & 0x1F))
#define JMP_X_NOT_Y(DEST)       (0x00A0 | ((DEST) & 0x1F))
#define JMP_PIN(X)              (0x00C0 | ((X) & 0x1F))

#define MOV_PINS_NULL           0xA003
#define MOV_X_PINS              0xA020
#define MOV_X_OSR               0xA027
#define MOV_PINDIRS_NULL        0xA063
#define MOV_PINDIRS_NOT_NULL    0xA06B
#define MOV_ISR_PINS            0xA0C0

#define NOP                     0xA042

#define OUT_PINS(NUM)           (0x6000 | ((NUM) & 0x1F))

#define PULL_BLOCK              0x80A0

#define PUSH_BLOCK              0x8020

#define SET_X(VALUE)            (0xE020 | ((VALUE) & 0x1F))
#define SET_Y(VALUE)            (0xE040 | ((VALUE) & 0x1F))

#define WAIT_IRQ_HIGH(X)        (0x20C0 | ((X) & 0x07))
#define WAIT_IRQ_HIGH_PREV(X)   (0x20C8 | ((X) & 0x07))
#define WAIT_IRQ_HIGH_NEXT(X)   (0x20D8 | ((X) & 0x07))
#define WAIT_IRQ_LOW(X)         (0x2040 | ((X) & 0x07))
#define WAIT_IRQ_LOW_PREV(X)    (0x2048 | ((X) & 0x07))
#define WAIT_IRQ_LOW_NEXT(X)    (0x2058 | ((X) & 0x07))
#define WAIT_PIN_HIGH(X)        (0x20A0 | ((X) & 0x1F))

// Clears IRQs for the specified PIO block
#define PIO_CLEAR_IRQ(X)        _Static_assert(X >= 0 && X <=2, "Invalid PIO block"); \
                                if (X == 0) {               \
                                    PIO0_IRQ = 0xFFFFFFFF;  \
                                } else if (X == 1) {        \
                                    PIO1_IRQ = 0xFFFFFFFF;  \
                                } else {                    \
                                    PIO2_IRQ = 0xFFFFFFFF;  \
                                }

// Clear all PIO IRQs
#define PIO_CLEAR_ALL_IRQS()    {                           \
                                    PIO0_IRQ = 0xFFFFFFFF;  \
                                    PIO1_IRQ = 0xFFFFFFFF;  \
                                    PIO2_IRQ = 0xFFFFFFFF;  \
                                }

#define PIO_INSTR_SCRATCH       uint32_t instr_scratch[32]
#define PIO_OFFSET(BLOCK)       uint8_t offset_##BLOCK = 0

// Macros for PIO SM register variables
#define PIO_SM_VARS(BLOCK, SM)  uint8_t first_instr_##BLOCK##_##SM = offset_##BLOCK; \
                                uint8_t start_##BLOCK##_##SM = offset_##BLOCK; \
                                uint8_t wrap_bottom_##BLOCK##_##SM = offset_##BLOCK; \
                                uint8_t wrap_top_##BLOCK##_##SM = offset_##BLOCK
#define PIO_SM_VAR_NEW(BLOCK, SM, VAR)  uint8_t var_##BLOCK##_##SM##_##VAR = offset_##BLOCK
#define PIO_SM_VAR(BLOCK, SM, VAR)      var_##BLOCK##_##SM##_##VAR
#define PIO_SM_SET_START(BLOCK, SM)     start_##BLOCK##_##SM = offset_##BLOCK
#define PIO_SM_SET_WRAP_BOTTOM(BLOCK, SM)   wrap_bottom_##BLOCK##_##SM = offset_##BLOCK
#define PIO_SM_SET_WRAP_TOP(BLOCK, SM)  wrap_top_##BLOCK##_##SM = offset_##BLOCK

#define PIO_ADD_INSTR(BLOCK, INST)      instr_scratch[offset_##BLOCK++] = INST

#define PIO_SM_CLKDIV_SET(BLOCK, SM, INT, FRAC)     uint32_t clkdiv_##BLOCK##_##SM = PIO_CLKDIV((INT), (FRAC))
#define PIO_SM_EXECCTRL_SET(BLOCK, SM, EXECCTRL)    uint32_t execctrl_##BLOCK##_##SM = (EXECCTRL)
#define PIO_SM_SHIFTCTRL_SET(BLOCK, SM, SHIFTCTRL)  uint32_t shiftctrl_##BLOCK##_##SM = (SHIFTCTRL)
#define PIO_SM_PINCTRL_SET(BLOCK, SM, PINCTRL)      uint32_t pinctrl_##BLOCK##_##SM = (PINCTRL)
#define PIO_SM_INSTR_SET(BLOCK, SM, INSTR)  {                                                 \
                                                volatile pio_sm_reg_t *sm_reg;                \
                                                if (BLOCK == 0) {                             \
                                                    sm_reg = PIO0_SM_REG(SM);                 \
                                                } else if (BLOCK == 1) {                      \
                                                    sm_reg = PIO1_SM_REG(SM);                 \
                                                } else {                                      \
                                                    sm_reg = PIO2_SM_REG(SM);                 \
                                                }                                             \
                                                sm_reg->instr = INSTR;                        \
                                            }

#define PIO_SM_COMMIT_REGS(BLOCK, SM)   {                                                     \
                                            volatile pio_sm_reg_t *sm_reg = PIO0_SM_REG(SM);  \
                                            sm_reg->clkdiv = clkdiv_##BLOCK##_##SM;           \
                                            sm_reg->execctrl =                                \
                                                execctrl_##BLOCK##_##SM |                     \
                                                PIO_WRAP_BOTTOM(wrap_bottom_##BLOCK##_##SM) | \
                                                PIO_WRAP_TOP(wrap_top_##BLOCK##_##SM);        \
                                            sm_reg->shiftctrl = shiftctrl_##BLOCK##_##SM;     \
                                            sm_reg->pinctrl = pinctrl_##BLOCK##_##SM;         \
                                        }
#define PIO_SM_JMP_TO_START(BLOCK, SM)  PIO_SM_INSTR_SET(BLOCK, SM, JMP(start_##BLOCK##_##SM))
#define PIO_WRITE_BLOCK(BLOCK)          for (int ii = 0; ii < offset_##BLOCK; ii++) {         \
                                            PIO0_INSTR_MEM(ii) = instr_scratch[ii];           \
                                        }

#if defined(DEBUG_LOGGING)
#define PIO_LOG_SM(BLOCK, SM, NAME)                     \
    pio_log_sm(                                         \
        NAME,                                           \
        BLOCK,                                          \
        SM,                                             \
        (uint32_t *)instr_scratch,                      \
        first_instr_##BLOCK##_##SM,                     \
        start_##BLOCK##_##SM                            \
    )
#else
#define PIO_LOG_SM(NAME, BLOCK, SM)
#endif // defined(DEBUG_LOGGING)

#endif // PIOASM_H