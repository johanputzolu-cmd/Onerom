// Copyright (C) 2026 Piers Finlayson <piers@piers.rocks>
//
// MIT License

#ifndef USB_PLUGIN_H
#define USB_PLUGIN_H

#include "plugin.h"

// Context structure for our plugin
typedef struct {
    ora_lookup_fn_t ora_lookup_fn;
    ora_log_fn_t log;
    ora_debug_log_fn_t debug;
    ora_err_log_fn_t err_log;
    uint32_t timer_ms;
} usb_plugin_context_t;

// Forward declaration of the context, which we define in usb_main.c
extern usb_plugin_context_t context;

// Logging macros
#if defined(DEBUG)
#undef DEBUG
#endif
#define DEBUG(...) do { \
    if (context.debug) { \
        context.debug(__VA_ARGS__); \
    } \
} while (0)

#if defined(LOG)
#undef LOG
#endif
#define LOG(...) do { \
    if (context.log) { \
        context.log(__VA_ARGS__); \
    } \
} while (0)

#if defined(ERR)
#undef ERR
#endif
#define ERR(...) do { \
    if (context.err_log) { \
        context.err_log(__VA_ARGS__); \
    } \
} while (0)

#endif // USB_PLUGIN_H