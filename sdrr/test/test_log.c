// Copyright (C) 2026 Piers Finlayson <piers@piers.rocks>
//
// MIT License

#include "stdio.h"
#include "stdarg.h"

void stub_log_v(const char* msg, va_list args) {
    vprintf(msg, args);
    printf("\n");
}

void stub_log(const char* msg, ...) {
    va_list args;
    va_start(args, msg);
    stub_log_v(msg, args);
    va_end(args);
}

void err_log(const char* msg, ...) {
    printf("ERROR: ");
    va_list args;
    va_start(args, msg);
    stub_log_v(msg, args);
    va_end(args);
}