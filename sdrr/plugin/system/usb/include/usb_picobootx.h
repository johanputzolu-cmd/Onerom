// Copyright (C) 2026 Piers Finlayson <piers@piers.rocks>
//
// MIT License

#if !defined(USB_PICOBOOTX_H)
#define USB_PICOBOOTX_H

#include "picobootx.h"

// Function prototypes
bool app_picoboot_control_xfer_cb(
    uint8_t rhport,
    uint8_t stage,
    tusb_control_request_t const *request
);
pb_status_t picoboot_read_validate(uint32_t addr, uint32_t size, void *ctx);

#endif // USB_PICOBOOTX_H