// Copyright (C) 2026 Piers Finlayson <piers@piers.rocks>
//
// MIT License

// The USB plugin's picobootx implementation.  This
// - plumbs in picobootx to the plugin itself
// - provides USB plugin specific picoboot protocol handling.

#include "usb_plugin.h"
#include "picobootx.h"
#include "picobootx_impl.h"
#include "usb_picobootx.h"

// Picboot state block.  Statically allocated here, but it is possible to
// allocate it at initialization dynamically if needed.
static uint32_t picoboot_state_buf[PICOBOOT_STATE_SIZE / 4];
#define picoboot_state ((pb_state_block_t *)picoboot_state_buf)

// Callbacks from the picoboot tinyusb vendor driver, which need to have
// picoboot_state passed in.  They just forward to the picoboot library
// callbacks.
void app_picoboot_rx_cb(uint32_t available_bytes) {
    picoboot_rx_cb(picoboot_state, available_bytes);
}
void app_picoboot_tx_cb(uint32_t sent_bytes) {
    picoboot_tx_cb(picoboot_state, sent_bytes);
}

// This callback is tud_vendor_control_xfer_cb, which is implemented by the
// application, and also needs to call into picoboot.
bool app_picoboot_control_xfer_cb(
    uint8_t rhport,
    uint8_t stage,
    tusb_control_request_t const *request
) {
    return picoboot_control_xfer_cb(picoboot_state, rhport, stage, request);
}

// Picoboot callback operations to provide implementations for the picoboot
// APIs.  We mostly use defaults.
static const picoboot_ops_t picoboot_ops = {
    .exclusive_access = picoboot_default_exclusive_access,
    .exit_xip = picoboot_default_exit_xip,
    .enter_xip = picoboot_default_enter_xip,
    .reboot2_prepare = picoboot_default_reboot2_prepare,
    .reboot2_execute = picoboot_default_reboot2_execute,
    .validate_read = picoboot_read_validate, 
    .read = picoboot_default_read,
    .otp_read = picoboot_default_otp_read,
    .get_info_sys = picoboot_default_get_info_sys,
};

// Initialize picobooy
void usb_picoboot_init(uint8_t ep_out, uint8_t ep_in) {
    picoboot_init(
        picoboot_state,
        &picoboot_ops,
        NULL,
        NULL,
        0,
        ep_out,
        ep_in, 
        &context
    );
}

void usb_picoboot_task(void) {
    picoboot_task(picoboot_state);
}

// Custom read validation callback for One ROM
pb_status_t picoboot_read_validate(uint32_t addr, uint32_t size, void *ctx) {
    (void)ctx;

    // First use the default validation
    pb_status_t st = picoboot_default_validate_read(addr, size, ctx);
    if (st != PB_STATUS_OK) {
        return st;
    }

    // For now don't do any other checking - picotool --debug looks beyond the
    // end of One ROM's flash, and expects a non-error response.
    return st;

    // Now add custom One ROM validation
#define RP2350_ROM_BASE    0x00000000u
#define RP2350_ROM_SIZE    0x00008000u  // 32KB
#define RP2350_FLASH_BASE  0x10000000u
#define RP2350_FLASH_SIZE  0x00200000u  // 2MB
#define RP2350_SRAM_BASE   0x20000000u
#define RP2350_SRAM_SIZE   0x00082000u  // 520KB

    // Validate entire range lies within a single valid region.
    // GCC dislikes >= ROM_BASE as it's 0, so always true
#pragma GCC diagnostic push
#pragma GCC diagnostic ignored "-Wtype-limits"
    bool valid =
        (addr >= RP2350_ROM_BASE   && (addr + size) <= (RP2350_ROM_BASE   + RP2350_ROM_SIZE))   ||
        (addr >= RP2350_FLASH_BASE && (addr + size) <= (RP2350_FLASH_BASE + RP2350_FLASH_SIZE)) ||
        (addr >= RP2350_SRAM_BASE  && (addr + size) <= (RP2350_SRAM_BASE  + RP2350_SRAM_SIZE));
#pragma GCC diagnostic pop

    if (!valid) {
        LOG("Invalid read request for One ROM: addr=0x%08x size=%u", addr, size);
        return PB_STATUS_INVALID_ADDRESS;
    }

    return PB_STATUS_OK;
}
