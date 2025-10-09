// Copyright (C) 2025 Piers Finlayson <piers@piers.rocks>
//
// MIT License

use crate::sdrr_types::{McuVariant, ServeAlg};

use onerom_config::hw::Board;

pub fn parse_mcu_variant(s: &str) -> Result<McuVariant, String> {
    McuVariant::try_from_str(s)
        .ok_or_else(|| format!("Invalid MCU variant: {}. Valid values are: f446rc, f446re, f411rc, f411re, f405rg, f401re, f401rb, f401rc for STM32, and rp2350 for Raspberry Pi", s))
}

pub fn parse_hw_rev(hw_rev: &str) -> Result<Board, String> {
    Board::try_from_str(hw_rev).ok_or_else(|| {
        format!(
            "Invalid hardware revision: {}. Use --list-hw-revs for options",
            hw_rev
        )
    })
}

pub fn parse_serve_alg(s: &str) -> Result<ServeAlg, String> {
    ServeAlg::try_from_str(s).ok_or_else(|| {
        format!(
            "Invalid serve algorithm: {}. Valid values are: default, a (2 CS 1 Addr), b (Addr on CS)",
            s
        )
    })
}
