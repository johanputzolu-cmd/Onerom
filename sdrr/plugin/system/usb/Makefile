# Copyright (C) 2026 Piers Finlayson <piers@piers.rocks>
#
# MIT License

TOOLCHAIN ?=
CC      := $(if $(TOOLCHAIN),$(TOOLCHAIN)/arm-none-eabi-gcc,arm-none-eabi-gcc)
OBJCOPY := $(if $(TOOLCHAIN),$(TOOLCHAIN)/arm-none-eabi-objcopy,arm-none-eabi-objcopy)
OBJDUMP := $(if $(TOOLCHAIN),$(TOOLCHAIN)/arm-none-eabi-objdump,arm-none-eabi-objdump)

ORA_DIR := ../../../ora
ORA_INCLUDE := -I$(ORA_DIR)
SDRR_INCLUDE := -I../../../include -I../../../../output
SEGGER_INCLUDE := -I../../../segger-rtt/RTT -I../../../segger-rtt/Config
PLUGIN_INCLUDE := -I.
PICO_SDK := ../../../../../pico-sdk
PICO_SDK_INCLUDE := \
    -I$(PICO_SDK)/build/generated/pico_base \
    -I$(PICO_SDK)/src/boards/include \
    -I$(PICO_SDK)/src/common/pico_base_headers/include \
    -I$(PICO_SDK)/src/rp2350/hardware_regs/include \
    -I$(PICO_SDK)/src/rp2350/hardware_structs/include \
    -I$(PICO_SDK)/src/rp2350/pico_platform/include \
    -I$(PICO_SDK)/src/rp2_common/hardware_base/include \
    -I$(PICO_SDK)/src/rp2_common/hardware_irq/include \
    -I$(PICO_SDK)/src/rp2_common/hardware_resets/include \
    -I$(PICO_SDK)/src/rp2_common/hardware_sync/include \
    -I$(PICO_SDK)/src/rp2_common/pico_platform_common/include \
    -I$(PICO_SDK)/src/rp2_common/pico_platform_compiler/include \
    -I$(PICO_SDK)/src/rp2_common/pico_platform_panic/include \
    -I$(PICO_SDK)/src/rp2_common/pico_platform_sections/include \
	-I$(PICO_SDK)/src/rp2_common/hardware_sync_spin_lock/include \
	-I$(PICO_SDK)/src/rp2_common/hardware_timer/include

BUILD_DIR := build

include tinyusb.mk
include picobootx/picobootx.mk

PICOBOOTX_SRC_C := $(addprefix picobootx/,$(PICOBOOTX_SRC_C))
PICOBOOTX_INCLUDE_DIRS := $(addprefix picobootx/,$(PICOBOOTX_INCLUDE_DIRS))

PLUGIN_BASE   := 0x10010000
PLUGIN_PREFIX := plugin_system_usb

ELF := $(BUILD_DIR)/$(PLUGIN_PREFIX).elf
BIN := $(BUILD_DIR)/$(PLUGIN_PREFIX).bin

SRC := usb_descriptors.c usb_picobootx.c usb_main.c

CFLAGS := -mcpu=cortex-m33 -mthumb -mfloat-abi=hard -mfpu=fpv5-sp-d16 \
          -O2 -Wall -Wextra -Werror \
          -ffunction-sections -fdata-sections \
          -MMD -MP \
          $(SDRR_INCLUDE) $(SEGGER_INCLUDE) $(ORA_INCLUDE) $(PLUGIN_INCLUDE) \
		  $(PICO_SDK_INCLUDE) \
          $(addprefix -I,$(TINYUSB_INCLUDE_DIRS)) \
		  $(addprefix -I,$(PICOBOOTX_INCLUDE_DIRS))

LDFLAGS := -nostdlib \
           -T usb.ld \
           -Wl,--defsym,PLUGIN_BASE=$(PLUGIN_BASE) \
           -Wl,--fatal-warnings \
           -Wl,-Map=$(BUILD_DIR)/$(PLUGIN_PREFIX).map

PLUGIN_OBJ := $(patsubst %.c,$(BUILD_DIR)/%.o,$(SRC))
TINYUSB_OBJ := $(patsubst %.c,$(BUILD_DIR)/%.o,$(TINYUSB_SRC_C))
PICOBOOTX_OBJ := $(patsubst %.c,$(BUILD_DIR)/%.o,$(PICOBOOTX_SRC_C))
ALL_OBJ     := $(PLUGIN_OBJ) $(TINYUSB_OBJ) $(PICOBOOTX_OBJ)

.PHONY: all clean

all: $(BIN)

$(BUILD_DIR)/%.o: %.c | $(BUILD_DIR)
	@mkdir -p $(dir $@)
	@echo "Compiling $<..."
	@$(CC) $(CFLAGS) -c $< -o $@

$(ELF): $(ALL_OBJ) $(ORA_DIR)/plugin.h $(ORA_DIR)/api.h $(ORA_DIR)/system.h $(ORA_DIR)/plugin.ld
	@echo "Linking..."
	@$(CC) $(CFLAGS) $(LDFLAGS) $(ALL_OBJ) -lc -lgcc -o $@

$(BIN): $(ELF)
	@echo "Converting ELF to binary..."
	@$(OBJCOPY) -O binary $< $@
	@$(OBJDUMP) -s -j .plugin_header $(ELF) > $(BUILD_DIR)/$(PLUGIN_PREFIX).dis
	@$(OBJDUMP) -d -S $(ELF) >> $(BUILD_DIR)/$(PLUGIN_PREFIX).dis

$(BUILD_DIR):
	@mkdir -p $@

clean:
	@echo "Cleaning build artifacts..."
	@rm -rf $(BUILD_DIR)

-include $(ALL_OBJ:.o=.d)