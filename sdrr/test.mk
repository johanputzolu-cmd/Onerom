# One ROM Test Makefile
# For native test builds only
MAKEFLAGS += --no-builtin-rules --no-builtin-variables

COLOUR_YELLOW := $(shell echo -e '\033[33m')
COLOUR_RESET := $(shell echo -e '\033[0m')

BUILD_DIR := build-test
BIN := $(BUILD_DIR)/onerom-test

# Output directory from sdrr-gen
GEN_OUTPUT_DIR ?= output
OUTPUT_DIR := ../$(GEN_OUTPUT_DIR)

# Include generated config
ifneq ($(wildcard $(OUTPUT_DIR)/generated.mk),)
  include $(OUTPUT_DIR)/generated.mk
else
  $(error sdrr-gen generated.mk not found. Run sdrr-gen first.)
endif

# Source files
SRCS := src/constants.c src/main.c src/rom_impl.c src/test.c src/utils.c \
        src/vector.c src/stm32f4.c src/rp235x.c src/piodma/pio.c \
        src/piodma/piorom.c src/piodma/pioram.c src/piodma/dma.c \
        test/stub_rp235x.c test/test_main.c test/test_log.c \
		apio/src/apio_dis.c epio/src/epio.c epio/src/epio_sram.c \
		apio/src/epio_apio.c epio/src/epio_gpio.c epio/src/epio_exec.c \
		apio/src/epio_fifo.c epio/src/epio_dma.c \
		wasm/src/wasm_main.c
OBJS := $(patsubst src/%.c,$(BUILD_DIR)/%.o,$(filter src/%,$(SRCS)))
OBJS += $(patsubst test/%.c,$(BUILD_DIR)/%.o,$(filter test/%,$(SRCS)))
OBJS += $(patsubst apio/src/%.c,$(BUILD_DIR)/%.o,$(filter apio/src/%,$(SRCS)))
OBJS += $(patsubst epio/src/%.c,$(BUILD_DIR)/%.o,$(filter epio/src/%,$(SRCS)))
OBJS += $(patsubst wasm/src/%.c,$(BUILD_DIR)/%.o,$(filter wasm/src/%,$(SRCS)))

# Generated files
ROMS_SRC := $(OUTPUT_DIR)/roms.c
ROMS_OBJ := $(BUILD_DIR)/roms.o
SDRR_CONFIG_SRC := $(OUTPUT_DIR)/sdrr_config.c
SDRR_CONFIG_OBJ := $(BUILD_DIR)/sdrr_config.o

VERSION_MAJOR := 0
VERSION_MINOR := 6
VERSION_PATCH := 4
BUILD_NUMBER := 0
GIT_COMMIT := $(shell git rev-parse --short HEAD 2>/dev/null || echo "unknown")

# Compile flags:
# - fsanitize=address -fno-omit-frame-pointer for debug builds
# - fshort-enums to ensure enums the same size as in firmware
CFLAGS := -DAPIO_EMULATION=1 -DTEST_BUILD=1 \
			$(EXTRA_C_FLAGS) -I include -I $(OUTPUT_DIR) -I include/test \
			-I apio/include -I epio/include \
			-DSDRR_VERSION_MAJOR=$(VERSION_MAJOR) -DSDRR_VERSION_MINOR=$(VERSION_MINOR) \
			-DSDRR_VERSION_PATCH=$(VERSION_PATCH) -DSDRR_BUILD_NUMBER=$(BUILD_NUMBER) \
			-DSDRR_GIT_COMMIT=\"$(GIT_COMMIT)\" \
			-DBOOT_LOGGING=1 -DDEBUG_LOGGING=1 \
			-g -O0 -Wall -Wextra -Werror -ffunction-sections -fdata-sections \
			-MMD -MP -fshort-enums -fsanitize=address -fno-omit-frame-pointer

# Linker flags:
# - fsanitize=address for debug builds
# - segalign 0x80000 to allow 512KB alignment (for ROM RAM table)
# - no_fixup_chains to make the 512KB alignement work on macOS
# - no_pie to avoid position independent executable which breaks alignment on macOS
LDFLAGS := -g -fsanitize=address 

# Targets
.PHONY: all clean run debug

all: $(BIN)
	@echo "Running One ROM test\n-----"
	@$(BIN)

$(BUILD_DIR):
	@mkdir -p $@

$(BUILD_DIR)/%.o: src/%.c | $(BUILD_DIR)
	@mkdir -p $(@D)
	@echo "- Compiling test/$<"
	@$(CC) $(CFLAGS) -c $< -o $@

$(BUILD_DIR)/%.o: test/%.c | $(BUILD_DIR)
	@mkdir -p $(@D)
	@echo "- Compiling test/$<"
	@$(CC) $(CFLAGS) -c $< -o $@

$(BUILD_DIR)/%.o: apio/src/%.c | $(BUILD_DIR)
	@mkdir -p $(@D)
	@echo "- Compiling $<"
	@$(CC) $(CFLAGS) -c $< -o $@

$(BUILD_DIR)/%.o: epio/src/%.c | $(BUILD_DIR)
	@mkdir -p $(@D)
	@echo "- Compiling $<"
	@$(CC) $(CFLAGS) -c $< -o $@

$(BUILD_DIR)/%.o: wasm/src/%.c | $(BUILD_DIR)
	@mkdir -p $(@D)
	@echo "- Compiling $<"
	@$(CC) $(CFLAGS) -c $< -o $@

$(ROMS_OBJ): $(ROMS_SRC) | $(BUILD_DIR)
	@echo "- Compiling test/$(ROMS_SRC)"
	@$(CC) $(CFLAGS) -c $< -o $@

$(SDRR_CONFIG_OBJ): $(SDRR_CONFIG_SRC) | $(BUILD_DIR)
	@echo "- Compiling test/$(SDRR_CONFIG_SRC)"
	@$(CC) $(CFLAGS) -c $< -o $@

$(BIN): $(OBJS) $(ROMS_OBJ) $(SDRR_CONFIG_OBJ)
	@echo "- Linking test"
	@$(CC) $(LDFLAGS) $^ -o $@

clean:
	@rm -rf $(BUILD_DIR)

-include $(OBJS:.o=.d) $(ROMS_OBJ:.o=.d) $(SDRR_CONFIG_OBJ:.o=.d)
