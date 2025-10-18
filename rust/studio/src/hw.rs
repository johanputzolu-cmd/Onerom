// Copyright (C) 2025 Piers Finlayson <piers@piers.rocks>
//
// MIT License

//! Contains One ROM hardware related types and functions

#[allow(unused_imports)]
use onerom_config::hw::{Board, MODELS, Model};
use onerom_config::mcu::Variant as McuVariant;

/// Information about hardware
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct HardwareInfo {
    pub board: Option<Board>,
    pub model: Option<Model>,
    pub mcu_variant: Option<McuVariant>,
}

impl HardwareInfo {
    pub fn is_complete(&self) -> bool {
        self.board.is_some() && self.model.is_some() && self.mcu_variant.is_some()
    }
}

impl std::fmt::Display for HardwareInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "HardwareInfo(board={:?}, model={:?}, mcu_variant={:?})",
            self.board.as_ref().map(|b| b.name()),
            self.model.as_ref().map(|m| m.name()),
            self.mcu_variant
        )
    }
}
