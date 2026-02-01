// Copyright (C) 2026 Piers Finlayson <piers@piers.rocks>
//
// MIT License

#ifndef PIODMA_H
#define PIODMA_H

#include "piodma/pioreg.h"
#include "piodma/pioasm.h"
#include "piodma/dmareg.h"

#define DMA_ENABLE()    RESET_RESET &= ~RESET_DMA;        \
                        while (!(RESET_DONE & RESET_DMA));

#endif // PIODMA_H