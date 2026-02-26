// Copyright (C) 2026 Piers Finlayson <piers@piers.rocks>
//
// MIT License

#ifndef PICOBOOT_H
#define PICOBOOT_H

#include <stdint.h>
#include <stdbool.h>
#include "tusb.h"

#ifdef __cplusplus
extern "C" {
#endif

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

#define PICOBOOT_MAGIC          0x431fd10bu
#define PICOBOOT_CMD_LEN        32u
#define PICOBOOT_ARGS_LEN       16u
#define PICOBOOT_STATUS_LEN     16u
#define PICOBOOT_DIR_IN         0x80u   // bit 7 of cmd_id set = host reads data

// Size of pb_state_block_t in bytes. Use this to allocate storage without
// needing to include picoboot_private.h. Verified by _Static_assert in
// picoboot_private.h.
#define PICOBOOT_STATE_SIZE     72u

// ---------------------------------------------------------------------------
// GET_INFO info types
// ---------------------------------------------------------------------------

typedef enum {
    PB_INFO_SYS              = 0x01,
    PB_INFO_PARTITION        = 0x02,
    PB_INFO_UF2_TARGET       = 0x03,
    PB_INFO_UF2_STATUS       = 0x04,
} pb_info_type_t;

// ---------------------------------------------------------------------------
// Status codes (returned via GET_COMMAND_STATUS control request)
// ---------------------------------------------------------------------------

typedef enum {
    PB_STATUS_OK                   = 0,
    PB_STATUS_UNKNOWN_CMD          = 1,
    PB_STATUS_INVALID_CMD_LENGTH   = 2,
    PB_STATUS_INVALID_TRANSFER_LEN = 3,
    PB_STATUS_INVALID_ADDRESS      = 4,
    PB_STATUS_BAD_ALIGNMENT        = 5,
    PB_STATUS_INTERLEAVED_WRITE    = 6,
    PB_STATUS_REBOOTING            = 7,
    PB_STATUS_UNKNOWN_ERROR        = 8,
    PB_STATUS_INVALID_STATE        = 9,
    PB_STATUS_NOT_PERMITTED        = 10,
    PB_STATUS_INVALID_ARG          = 11,
    PB_STATUS_BUFFER_TOO_SMALL     = 12,
    PB_STATUS_PRECONDITION_NOT_MET = 13,
    PB_STATUS_MODIFIED_DATA        = 14,
    PB_STATUS_INVALID_DATA         = 15,
    PB_STATUS_NOT_FOUND            = 16,
    PB_STATUS_UNSUPPORTED_MOD      = 17,
} pb_status_t;

// ---------------------------------------------------------------------------
// Wire structs
// ---------------------------------------------------------------------------

// Full 32-byte command packet received on BULK_OUT
typedef struct __attribute__((packed)) {
    uint32_t magic;
    uint32_t token;
    uint8_t  cmd_id;
    uint8_t  cmd_size;
    uint16_t reserved;
    uint32_t transfer_len;
    uint8_t  args[PICOBOOT_ARGS_LEN];
} picoboot_cmd_t;

// 16-byte status packet returned via GET_COMMAND_STATUS control request
typedef struct __attribute__((packed)) {
    uint32_t token;
    uint32_t status_code;
    uint8_t  cmd_id;
    uint8_t  in_progress;
    uint8_t  reserved[6];
} picoboot_status_t;

// ---------------------------------------------------------------------------
// Args structs — overlaid onto picoboot_cmd_t.args
// ---------------------------------------------------------------------------

typedef struct __attribute__((packed)) {
    uint8_t exclusive;  // 0=NOT_EXCLUSIVE, 1=EXCLUSIVE, 2=EXCLUSIVE_AND_EJECT
} pb_exclusive_access_args_t;

typedef struct __attribute__((packed)) {
    uint32_t addr;
    uint32_t size;
} pb_addr_size_args_t;   // shared by READ, WRITE, FLASH_ERASE

typedef struct __attribute__((packed)) {
    uint32_t flags;
    uint32_t delay_ms;
    uint32_t p0;
    uint32_t p1;
} pb_reboot2_args_t;

typedef struct __attribute__((packed)) {
    uint8_t  info_type;   // pb_info_type_t
    uint8_t  reserved[3];
    uint32_t param0;      // flags for INFO_SYS; flags_and_partition for PARTITION;
                          // family_id for UF2_TARGET
} pb_get_info_args_t;

typedef struct __attribute__((packed)) {
    uint16_t row;
    uint16_t row_count;
    uint8_t  ecc;         // 0=raw (32 bits/row), 1=ECC (16 bits/row)
} pb_otp_args_t;          // shared by OTP_READ and OTP_WRITE

// ---------------------------------------------------------------------------
// Callback interfaces
// ---------------------------------------------------------------------------

// Standard PICOBOOT ops (magic == PICOBOOT_MAGIC).
//
// No-data-phase callbacks return pb_status_t directly.
//
// IN callbacks (read, get_info_sys, otp_read) receive a buffer to fill and
// must set *bytes_written on PB_STATUS_OK.  The library calls get_info_sys
// once per requested flag with a buffer sized to that flag's known word
// count; the callback must call get_sys_info() and return only the data
// words (not the leading supported-flags word from get_sys_info).
//
// OUT callbacks (write, otp_write) receive the fully-accumulated data buffer.
// write() will not be called if flash_write_buf was NULL at picoboot_init();
// the library returns PB_STATUS_NOT_PERMITTED automatically in that case.
//
// post_reboot2 is called after the ZLP for REBOOT2 has been queued, so the
// integrator can defer the actual reboot until USB has quiesced. May be NULL
// if the reboot2 callback handles deferral itself.
typedef struct {
    // No data phase
    pb_status_t (*exclusive_access)(const pb_exclusive_access_args_t *args, void *ctx);
    pb_status_t (*flash_erase)     (const pb_addr_size_args_t *args, void *ctx);
    pb_status_t (*reboot2)         (const pb_reboot2_args_t *args, void *ctx);

    // OUT data phase (host -> device); write buffer is always 256 bytes
    pb_status_t (*write)           (const pb_addr_size_args_t *args,
                                    const uint8_t *buf, uint32_t len, void *ctx);
    pb_status_t (*otp_write)       (const pb_otp_args_t *args,
                                    const uint8_t *buf, uint32_t len, void *ctx);

    // IN data phase (device -> host)
    pb_status_t (*read)            (const pb_addr_size_args_t *args,
                                    uint8_t *buf, uint32_t buf_len,
                                    uint32_t *bytes_written, void *ctx);
    pb_status_t (*get_info_sys)    (uint32_t flags,
                                    uint8_t *buf, uint32_t buf_len,
                                    uint32_t *bytes_written, void *ctx);
    pb_status_t (*otp_read)        (const pb_otp_args_t *args,
                                    uint8_t *buf, uint32_t buf_len,
                                    uint32_t *bytes_written, void *ctx);

    // Post-ZLP hook for REBOOT2 (may be NULL)
    void        (*post_reboot2)    (const pb_reboot2_args_t *args, void *ctx);
} picoboot_ops_t;

// Custom / extended command dispatch (alternative magic value).
// The library handles framing, token tracking, ZLP, and stall.
// The integrator owns all command routing and data buffer management.
// For IN-direction commands, bytes_written must be set on PB_STATUS_OK.
typedef struct {
    uint32_t    magic;
    pb_status_t (*dispatch)(const picoboot_cmd_t *cmd,
                            uint8_t *buf, uint32_t buf_len,
                            uint32_t *bytes_written, void *ctx);
} picoboot_custom_ops_t;

// ---------------------------------------------------------------------------
// State block — integrator allocates, library owns contents
// ---------------------------------------------------------------------------

// Opaque to integrators. Allocate PICOBOOT_STATE_SIZE bytes with at least
// 4-byte alignment (required for OTP buffer reuse via otp_access()).
// Example static allocation:
//   static uint32_t picoboot_state_buf[PICOBOOT_STATE_SIZE / 4];
//   #define picoboot_state ((pb_state_block_t *)picoboot_state_buf)
typedef struct pb_state_block pb_state_block_t;

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

// Initialise the library.
//   state           : caller-allocated state block (PICOBOOT_STATE_SIZE bytes,
//                     4-byte aligned)
//   ops             : standard PICOBOOT command callbacks
//   custom          : extended magic dispatch; may be NULL
//   flash_write_buf : 256-byte, 4-byte-aligned buffer for write accumulation;
//                     NULL disables WRITE and OTP_WRITE (PB_STATUS_NOT_PERMITTED)
//   ep_out          : BULK OUT endpoint address (from your USB descriptor)
//   ep_in           : BULK IN endpoint address (from your USB descriptor)
//   ctx             : passed verbatim to all callbacks
void picoboot_init(pb_state_block_t            *state,
                   const picoboot_ops_t        *ops,
                   const picoboot_custom_ops_t *custom,
                   uint8_t                     *flash_write_buf,
                   uint8_t                      rhport,
                   uint8_t                      ep_out,
                   uint8_t                      ep_in,
                   void                        *ctx);

// Call from your main loop / plugin task alongside tud_task().
void picoboot_task(pb_state_block_t *state);

// Wire into tud_vendor_control_xfer_cb() — return its result directly.
bool picoboot_control_xfer_cb(pb_state_block_t             *state,
                               uint8_t                       rhport,
                               uint8_t                       stage,
                               tusb_control_request_t const *req);

#ifdef __cplusplus
}
#endif

#endif // PICOBOOT_H