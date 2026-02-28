// Copyright (C) 2026 Piers Finlayson <piers@piers.rocks>
//
// MIT License

#if !defined(USB_PICOBOOT_H)
#define USB_PICOBOOT_H

// Function prototypes
pb_status_t picoboot_read_validate(uint32_t addr, uint32_t size, void *ctx);

#endif // USB_PICOBOOT_H