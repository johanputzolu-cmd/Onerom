# One ROM Test Makefile
# For native test builds only
MAKEFLAGS += --no-builtin-rules --no-builtin-variables

COLOUR_YELLOW := $(shell echo -e '\033[33m')
COLOUR_RESET := $(shell echo -e '\033[0m')

# Output directory from sdrr-gen
GEN_OUTPUT_DIR ?= output
OUTPUT_DIR := ../$(GEN_OUTPUT_DIR)

# Include generated config
ifneq ($(wildcard $(OUTPUT_DIR)/generated.mk),)
  include $(OUTPUT_DIR)/generated.mk
else
  $(error sdrr-gen generated.mk not found. Run sdrr-gen first.)
endif

# WASM build configuration
WASM_BUILD_DIR := build-wasm
WASM_CC := emcc
WASM_BIN := $(WASM_BUILD_DIR)/onerom-wasm.html

# Source files
SRCS := src/constants.c src/main.c src/rom_impl.c src/test.c src/utils.c \
        src/vector.c src/stm32f4.c src/rp235x.c src/piodma/pio.c \
        src/piodma/piorom.c src/piodma/pioram.c src/piodma/dma.c \
        test/stub_rp235x.c test/test_log.c \
		epio/src/epio.c epio/src/epio_sram.c \
		epio/src/epio_apio.c epio/src/epio_gpio.c epio/src/epio_exec.c \
		epio/src/epio_fifo.c epio/src/epio_dma.c \
		wasm/src/wasm_main.c

# WASM object files (same sources, different build dir)
WASM_OBJS := $(patsubst src/%.c,$(WASM_BUILD_DIR)/%.o,$(filter src/%,$(SRCS)))
WASM_OBJS += $(patsubst test/%.c,$(WASM_BUILD_DIR)/%.o,$(filter test/%,$(SRCS)))
WASM_OBJS += $(patsubst epio/src/%.c,$(WASM_BUILD_DIR)/%.o,$(filter epio/src/%,$(SRCS)))
WASM_OBJS += $(patsubst wasm/src/%.c,$(WASM_BUILD_DIR)/%.o,$(filter wasm/src/%,$(SRCS)))
WASM_ROMS_OBJ := $(WASM_BUILD_DIR)/roms.o
WASM_SDRR_CONFIG_OBJ := $(WASM_BUILD_DIR)/sdrr_config.o

# Generated files
ROMS_SRC := $(OUTPUT_DIR)/roms.c
ROMS_OBJ := $(BUILD_DIR)/roms.o
SDRR_CONFIG_SRC := $(OUTPUT_DIR)/sdrr_config.c
SDRR_CONFIG_OBJ := $(BUILD_DIR)/sdrr_config.o

# Web files
WEB_FILES := wasm/web/index.html wasm/web/logic-analyzer.js wasm/web/style.css

VERSION_MAJOR := 0
VERSION_MINOR := 6
VERSION_PATCH := 4
BUILD_NUMBER := 0
GIT_COMMIT := $(shell git rev-parse --short HEAD 2>/dev/null || echo "unknown")

# WASM-specific flags (no sanitizer, add EPIO_WASM define)
WASM_CFLAGS := -DAPIO_EMULATION=1 -DTEST_BUILD=1 -DEPIO_WASM \
			$(EXTRA_C_FLAGS) -I include -I $(OUTPUT_DIR) -I include/test \
			-I apio/include -I epio/include \
			-DSDRR_VERSION_MAJOR=$(VERSION_MAJOR) -DSDRR_VERSION_MINOR=$(VERSION_MINOR) \
			-DSDRR_VERSION_PATCH=$(VERSION_PATCH) -DSDRR_BUILD_NUMBER=$(BUILD_NUMBER) \
			-DSDRR_GIT_COMMIT=\"$(GIT_COMMIT)\" \
			-DBOOT_LOGGING=1 -DDEBUG_LOGGING=1 \
			-g -O0 -Wall -Wextra -Werror -ffunction-sections -fdata-sections \
			-MMD -MP -fshort-enums

# Emscripten linker flags
WASM_LDFLAGS := -s WASM=1 \
				-s EXPORTED_RUNTIME_METHODS='["ccall","cwrap"]' \
				-s EXPORTED_FUNCTIONS='["_malloc","_free",\
					"_onerom_init","_onerom_step",\
					"_onerom_drive_pins","_onerom_release_pins",\
					"_onerom_read_data","_onerom_read_gpios",\
					"_onerom_get_addr_pin","_onerom_get_data_pin",\
					"_onerom_get_cs1_pin","_onerom_get_cs2_pin",\
					"_onerom_get_cs3_pin","_onerom_get_x1_pin","_onerom_get_x2_pin",\
					"_epio_init","_epio_free","_epio_set_gpiobase",\
					"_epio_set_sm_reg","_epio_get_sm_reg","_epio_enable_sm",\
					"_epio_set_instr","_epio_get_instr","_epio_step_cycles",\
					"_epio_get_cycle_count","_epio_reset_cycle_count",\
					"_epio_wait_tx_fifo","_epio_tx_fifo_depth","_epio_rx_fifo_depth",\
					"_epio_pop_rx_fifo","_epio_push_tx_fifo","_epio_push_rx_fifo",\
					"_epio_drive_gpios_ext","_epio_read_gpios_ext",\
					"_epio_get_gpio_input","_epio_init_gpios",\
					"_epio_set_gpio_input","_epio_set_gpio_output",\
					"_epio_set_gpio_input_level","_epio_set_gpio_output_level",\
					"_epio_read_pin_states","_epio_read_driven_pins",\
					"_epio_sram_read_byte","_epio_sram_set",\
					"_epio_sram_read_halfword","_epio_sram_read_word",\
					"_epio_sram_write_byte","_epio_sram_write_halfword","_epio_sram_write_word"]' \
				-s ALLOW_MEMORY_GROWTH=1

# Targets
.PHONY: all clean run debug web copy-web serve clean-apio-src apio

all:  $(WASM_BIN) copy-web
	@echo "WASM build complete: $(WASM_BIN)"

# Copy web files to build directory
copy-web: $(WEB_FILES)
	@echo "- Copying web files to $(WASM_BUILD_DIR)"
	@cp wasm/web/index.html $(WASM_BUILD_DIR)/
	@cp wasm/web/logic-analyzer.js $(WASM_BUILD_DIR)/
	@cp wasm/web/style.css $(WASM_BUILD_DIR)/

# Alias for copy-web
web: copy-web

# Alias for serve
run: serve

# Run web server
serve: all
	@echo "$(COLOUR_YELLOW)Starting web server on http://localhost:8000 $(COLOUR_RESET)"
	@echo "$(COLOUR_YELLOW)Open http://localhost:8000/index.html in your browser $(COLOUR_RESET)"
	@cd $(WASM_BUILD_DIR) && python3 -m http.server 8000

apio:
	@if [ ! -d "$@" ]; then \
		git clone https://github.com/piersfinlayson/apio.git; \
	fi

$(WASM_BUILD_DIR):
	@mkdir -p $@

$(WASM_BUILD_DIR)/%.o: src/%.c | $(WASM_BUILD_DIR) apio
	@mkdir -p $(@D)
	@echo "- Compiling WASM $<"
	@$(WASM_CC) $(WASM_CFLAGS) -c $< -o $@

$(WASM_BUILD_DIR)/%.o: test/%.c | $(WASM_BUILD_DIR) apio
	@mkdir -p $(@D)
	@echo "- Compiling WASM $<"
	@$(WASM_CC) $(WASM_CFLAGS) -c $< -o $@

$(WASM_BUILD_DIR)/%.o: epio/src/%.c | $(WASM_BUILD_DIR)
	@mkdir -p $(@D)
	@echo "- Compiling WASM $<"
	@$(WASM_CC) $(WASM_CFLAGS) -c $< -o $@

$(WASM_BUILD_DIR)/%.o: wasm/src/%.c | $(WASM_BUILD_DIR) apio
	@mkdir -p $(@D)
	@echo "- Compiling WASM $<"
	@$(WASM_CC) $(WASM_CFLAGS) -c $< -o $@

$(WASM_ROMS_OBJ): $(ROMS_SRC) | $(WASM_BUILD_DIR)
	@echo "- Compiling WASM $(ROMS_SRC)"
	@$(WASM_CC) $(WASM_CFLAGS) -c $< -o $@

$(WASM_SDRR_CONFIG_OBJ): $(SDRR_CONFIG_SRC) | $(WASM_BUILD_DIR)
	@echo "- Compiling WASM $(SDRR_CONFIG_SRC)"
	@$(WASM_CC) $(WASM_CFLAGS) -c $< -o $@

$(WASM_BIN): $(WASM_OBJS) $(WASM_ROMS_OBJ) $(WASM_SDRR_CONFIG_OBJ)
	@echo "- Linking WASM"
	@$(WASM_CC) $(WASM_LDFLAGS) $^ -o $@

clean-apio-src:
	rm -rf apio/

clean: clean-apio-src
	@rm -rf $(BUILD_DIR)

-include $(WASM_OBJS:.o=.d) $(WASM_ROMS_OBJ:.o=.d) $(WASM_SDRR_CONFIG_OBJ:.o=.d)