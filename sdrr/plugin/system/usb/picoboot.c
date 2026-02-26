// Copyright (C) 2026 Piers Finlayson <piers@piers.rocks>
//
// MIT License

#include <string.h>
#include "device/usbd_pvt.h"   // usbd_edpt_stall(), usbd_edpt_clear_stall()
#include "picoboot_private.h"

// ---------------------------------------------------------------------------
// GET_INFO: static flag -> word count table
//
// Each entry maps a single INFO_SYS flag bit to the number of data words
// get_sys_info() returns for that flag (not counting the leading
// supported-flags word).  Used to pre-compute the count word that heads the
// PICOBOOT GET_INFO response before any flag data is sent.
//
// Flags not listed here are treated as returning 0 words (unsupported).
// Update this table if the bootrom adds new INFO_SYS flags.
// ---------------------------------------------------------------------------

typedef struct {
    uint32_t flag;
    uint8_t  word_count;
} pb_info_flag_entry_t;

static const pb_info_flag_entry_t k_info_flag_table[] = {
    { 0x0001u, 3u },   // CHIP_INFO
    { 0x0002u, 1u },   // CRITICAL
    { 0x0004u, 1u },   // CPU_INFO
    { 0x0008u, 1u },   // FLASH_DEV_INFO
    { 0x0010u, 4u },   // BOOT_RANDOM
    { 0x0020u, 0u },   // NONCE (not supported)
    { 0x0040u, 4u },   // BOOT_INFO
};

#define INFO_FLAG_TABLE_COUNT \
    (sizeof(k_info_flag_table) / sizeof(k_info_flag_table[0]))

// Returns total data word count for all flags set in `requested`.
// Flags not in the table contribute 0.
static uint32_t pb_info_count_words(uint32_t requested) {
    uint32_t total = 0u;
    for (uint32_t i = 0u; i < INFO_FLAG_TABLE_COUNT; i++) {
        if (requested & k_info_flag_table[i].flag) {
            total += k_info_flag_table[i].word_count;
        }
    }
    return total;
}

// Returns the word count for a single flag (lowest set bit of `flag`).
static uint8_t pb_info_words_for_flag(uint32_t flag) {
    for (uint32_t i = 0u; i < INFO_FLAG_TABLE_COUNT; i++) {
        if (k_info_flag_table[i].flag == flag) {
            return k_info_flag_table[i].word_count;
        }
    }
    return 0u;
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

void pb_set_status(pb_state_block_t *s, pb_status_t code, bool in_progress) {
    s->status.token       = s->token;
    s->status.status_code = (uint32_t)code;
    s->status.cmd_id      = s->cmd_id;
    s->status.in_progress = in_progress ? 1u : 0u;
    memset(s->status.reserved, 0, sizeof(s->status.reserved));
}

void pb_stall(pb_state_block_t *s, pb_status_t code) {
    pb_set_status(s, code, false);
    usbd_edpt_stall(s->rhport, s->ep_out);
    usbd_edpt_stall(s->rhport, s->ep_in);
    s->state = PB_STATE_STALLED;
}

void pb_send_zlp(pb_state_block_t *s) {
    // A zero-length write followed by flush causes TinyUSB to emit a ZLP on
    // the IN endpoint.  We move to AWAIT_ZLP and poll for TX drain before
    // taking any post-command action (e.g. REBOOT2).
    tud_vendor_write(NULL, 0u);
    tud_vendor_write_flush();
    pb_set_status(s, PB_STATUS_OK, false);
    s->state = PB_STATE_AWAIT_ZLP;
}

// ---------------------------------------------------------------------------
// Command validation helpers
// ---------------------------------------------------------------------------

// Validate fields that are common to all commands regardless of which magic
// was matched.  Returns PB_STATUS_OK if valid.
static pb_status_t pb_validate_cmd(const picoboot_cmd_t *cmd,
                                   uint8_t expected_cmd_size,
                                   uint32_t expected_transfer_len) {
    if (cmd->cmd_size != expected_cmd_size) {
        return PB_STATUS_INVALID_CMD_LENGTH;
    }
    if (cmd->transfer_len != expected_transfer_len) {
        return PB_STATUS_INVALID_TRANSFER_LEN;
    }
    return PB_STATUS_OK;
}

// ---------------------------------------------------------------------------
// Command dispatch
// ---------------------------------------------------------------------------

static void pb_dispatch_reboot2(pb_state_block_t *s, const picoboot_cmd_t *cmd) {
    pb_status_t st = pb_validate_cmd(cmd, 0x10u, 0x00000000u);
    if (st != PB_STATUS_OK) {
        pb_stall(s, st);
        return;
    }

    if (!s->ops->reboot2) {
        pb_stall(s, PB_STATUS_UNKNOWN_CMD);
        return;
    }

    // Copy args before dispatching — they must outlive this call so that
    // post_reboot2 can access them after the ZLP has been sent.
    const pb_reboot2_args_t *args = (const pb_reboot2_args_t *)cmd->args;
    s->reboot2_args = *args;

    st = s->ops->reboot2(&s->reboot2_args, s->ctx);
    if (st != PB_STATUS_OK) {
        pb_stall(s, st);
        return;
    }

    pb_send_zlp(s);
}

static void pb_dispatch_get_info(pb_state_block_t *s, const picoboot_cmd_t *cmd) {
    const pb_get_info_args_t *args = (const pb_get_info_args_t *)cmd->args;

    // transfer_len must be non-zero, a multiple of 4, and < 256 per spec
    if (cmd->transfer_len == 0u ||
        (cmd->transfer_len & 0x3u) != 0u ||
        cmd->transfer_len > 255u) {
        pb_stall(s, PB_STATUS_INVALID_TRANSFER_LEN);
        return;
    }

    if (args->info_type != (uint8_t)PB_INFO_SYS) {
        // Only INFO_SYS supported; all others are stubs
        pb_stall(s, PB_STATUS_UNKNOWN_CMD);
        return;
    }

    if (!s->ops->get_info_sys) {
        pb_stall(s, PB_STATUS_UNKNOWN_CMD);
        return;
    }

    // Set up GET_INFO streaming state
    s->xfer.get_info.remaining_flags    = args->param0;
    s->xfer.get_info.transfer_remaining = cmd->transfer_len;
    s->xfer.get_info.header_sent        = false;
    s->state = PB_STATE_DATA_IN;
    // Actual data streaming happens in pb_task_data_in()
}

static void pb_dispatch_cmd(pb_state_block_t *s, const picoboot_cmd_t *cmd) {
    // Capture token and cmd_id for GET_COMMAND_STATUS before any early return
    s->token  = cmd->token;
    s->cmd_id = cmd->cmd_id;

    switch ((pb_cmd_id_t)cmd->cmd_id) {
        case PB_CMD_REBOOT2:
            pb_dispatch_reboot2(s, cmd);
            break;

        case PB_CMD_GET_INFO:
            pb_dispatch_get_info(s, cmd);
            break;

        // No-ops on RP2350
        case PB_CMD_EXIT_XIP:
        case PB_CMD_ENTER_XIP:
            pb_send_zlp(s);
            break;

        // Not supported on RP2350; stall cleanly
        case PB_CMD_REBOOT:
        case PB_CMD_EXEC:
        case PB_CMD_VECTORIZE_FLASH:
            pb_stall(s, PB_STATUS_UNKNOWN_CMD);
            break;

        // Supported but not yet implemented
        case PB_CMD_EXCLUSIVE_ACCESS:
        case PB_CMD_FLASH_ERASE:
        case PB_CMD_WRITE:
        case PB_CMD_READ:
        case PB_CMD_OTP_READ:
        case PB_CMD_OTP_WRITE:
            pb_stall(s, PB_STATUS_UNKNOWN_CMD);
            break;

        default:
            pb_stall(s, PB_STATUS_UNKNOWN_CMD);
            break;
    }
}

// ---------------------------------------------------------------------------
// State machine handlers called from picoboot_task()
// ---------------------------------------------------------------------------

static void pb_task_idle(pb_state_block_t *s) {
    if (tud_vendor_available() < PICOBOOT_CMD_LEN) {
        return;
    }

    picoboot_cmd_t cmd;
    uint32_t n = tud_vendor_read(&cmd, sizeof(cmd));
    if (n != PICOBOOT_CMD_LEN) {
        // Partial read — shouldn't happen if available() >= 32, but guard anyway
        pb_stall(s, PB_STATUS_UNKNOWN_ERROR);
        return;
    }

    // Magic discrimination: route to standard or custom dispatch
    if (cmd.magic == PICOBOOT_MAGIC) {
        pb_dispatch_cmd(s, &cmd);
    } else if (s->custom && cmd.magic == s->custom->magic) {
        s->token  = cmd.token;
        s->cmd_id = cmd.cmd_id;
        uint32_t bytes_written = 0u;
        pb_status_t st = s->custom->dispatch(&cmd, NULL, 0u, &bytes_written, s->ctx);
        if (st != PB_STATUS_OK) {
            pb_stall(s, st);
        } else {
            pb_send_zlp(s);
        }
    } else {
        // Unknown magic — stall.  No valid token to report.
        s->token  = cmd.token;
        s->cmd_id = cmd.cmd_id;
        pb_stall(s, PB_STATUS_UNKNOWN_CMD);
    }
}

static void pb_task_data_in(pb_state_block_t *s) {
    if (s->cmd_id != (uint8_t)PB_CMD_GET_INFO) {
        // Other IN commands not yet implemented
        pb_stall(s, PB_STATUS_UNKNOWN_ERROR);
        return;
    }

    pb_in_get_info_t *gi = &s->xfer.get_info;

    // Send the leading count word first (total significant data words)
    if (!gi->header_sent) {
        if (tud_vendor_write_available() < sizeof(uint32_t)) {
            return;  // no space yet; try next task() call
        }
        uint32_t word_count = pb_info_count_words(gi->remaining_flags);
        tud_vendor_write((const uint8_t *)&word_count, sizeof(word_count));
        tud_vendor_write_flush();
        gi->transfer_remaining -= sizeof(uint32_t);
        gi->header_sent = true;

        if (gi->transfer_remaining == 0u || gi->remaining_flags == 0u) {
            pb_send_zlp(s);
            return;
        }
    }

    // Process one flag per task() call, looping while TX space permits
    while (gi->remaining_flags != 0u && gi->transfer_remaining > 0u) {
        // Isolate the lowest set flag
        uint32_t flag = gi->remaining_flags & (~gi->remaining_flags + 1u);
        uint8_t  wc   = pb_info_words_for_flag(flag);

        if (wc == 0u) {
            // Flag is in the table but returns no words (unsupported by bootrom)
            gi->remaining_flags &= ~flag;
            continue;
        }

        uint32_t data_bytes = (uint32_t)wc * sizeof(uint32_t);

        if (tud_vendor_write_available() < data_bytes) {
            return;  // no space; try next task() call
        }

        // Stack buffer: largest single flag is 4 words = 16 bytes
        uint8_t  buf[16];
        uint32_t bytes_written = 0u;

        pb_status_t st = s->ops->get_info_sys(flag, buf, data_bytes, &bytes_written, s->ctx);
        if (st != PB_STATUS_OK) {
            pb_stall(s, st);
            return;
        }

        tud_vendor_write(buf, bytes_written);
        tud_vendor_write_flush();

        gi->remaining_flags    &= ~flag;
        gi->transfer_remaining  = (gi->transfer_remaining > bytes_written)
                                  ? gi->transfer_remaining - bytes_written
                                  : 0u;
    }

    // Zero-pad any remaining transfer_len bytes the host is expecting
    while (gi->transfer_remaining > 0u) {
        uint32_t space = tud_vendor_write_available();
        if (space == 0u) {
            return;
        }
        uint8_t  zeroes[16] = {0};
        uint32_t chunk = gi->transfer_remaining < sizeof(zeroes)
                         ? gi->transfer_remaining : sizeof(zeroes);
        if (chunk > space) chunk = space;
        tud_vendor_write(zeroes, chunk);
        tud_vendor_write_flush();
        gi->transfer_remaining -= chunk;
    }

    pb_send_zlp(s);
}

static void pb_task_await_zlp(pb_state_block_t *s) {
    // Poll until the TX FIFO is fully drained — at that point the ZLP has
    // been sent and it is safe to take any post-command action.
    if (tud_vendor_write_available() < CFG_TUD_VENDOR_TX_BUFSIZE) {
        return;
    }

    if (s->cmd_id == (uint8_t)PB_CMD_REBOOT2) {
        if (s->ops->post_reboot2) {
            s->ops->post_reboot2(&s->reboot2_args, s->ctx);
        }
    }

    s->state = PB_STATE_IDLE;
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

void picoboot_init(pb_state_block_t            *state,
                   const picoboot_ops_t        *ops,
                   const picoboot_custom_ops_t *custom,
                   uint8_t                     *flash_write_buf,
                   uint8_t                      rhport,
                   uint8_t                      ep_out,
                   uint8_t                      ep_in,
                   void                        *ctx) {
    memset(state, 0, sizeof(*state));
    state->ops             = ops;
    state->custom          = custom;
    state->flash_write_buf = flash_write_buf;
    state->rhport          = rhport;
    state->ep_out          = ep_out;
    state->ep_in           = ep_in;
    state->ctx             = ctx;
    state->state           = PB_STATE_IDLE;
}

void picoboot_task(pb_state_block_t *state) {
    switch (state->state) {
        case PB_STATE_IDLE:
            pb_task_idle(state);
            break;
        case PB_STATE_DATA_IN:
            pb_task_data_in(state);
            break;
        case PB_STATE_AWAIT_ZLP:
            pb_task_await_zlp(state);
            break;
        case PB_STATE_DATA_OUT:
            // Not yet implemented
            break;
        case PB_STATE_STALLED:
            // Nothing to do — pb_control_xfer_cb handles unstall on
            // INTERFACE_RESET or after GET_COMMAND_STATUS is serviced
            break;
    }
}

bool picoboot_control_xfer_cb(pb_state_block_t             *state,
                               uint8_t                       rhport,
                               uint8_t                       stage,
                               tusb_control_request_t const *req) {
    // Only handle class/vendor requests directed at our interface
    if ((req->bmRequestType_bit.type != TUSB_REQ_TYPE_CLASS &&
         req->bmRequestType_bit.type != TUSB_REQ_TYPE_VENDOR) ||
        req->bmRequestType_bit.recipient != TUSB_REQ_RCPT_INTERFACE) {
        return false;
    }

    switch (req->bRequest) {

        case PICOBOOT_BREQUEST_INTERFACE_RESET:
            if (stage == CONTROL_STAGE_SETUP) {
                // Reset state machine and unstall endpoints if needed
                if (state->state == PB_STATE_STALLED) {
                    usbd_edpt_clear_stall(rhport, state->ep_out);
                    usbd_edpt_clear_stall(rhport, state->ep_in);
                }
                state->state = PB_STATE_IDLE;
                return tud_control_status(rhport, req);
            }
            return true;

        case PICOBOOT_BREQUEST_GET_CMD_STATUS:
            if (stage == CONTROL_STAGE_SETUP) {
                return tud_control_xfer(rhport, req,
                                        &state->status,
                                        sizeof(state->status));
            }
            if (stage == CONTROL_STAGE_ACK) {
                // Host has read the status. If we were stalled and the status
                // code indicates completion, unstall and return to IDLE.
                // The host is responsible for issuing INTERFACE_RESET before
                // retrying after an error; we do not auto-unstall here.
            }
            return true;

        default:
            return false;
    }
}