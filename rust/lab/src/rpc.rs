//! One ROM Lab firmware - RPC objects

// Copyright (c) 2025 Piers Finlayson <piers@piers.rocks>
//
// MIT licence

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum Command {
    Ping = 0x0000_0000,
    ReadRom = 0x0000_0001,
    Unknown = 0xFFFF_FFFF,
}

impl From<u32> for Command {
    fn from(value: u32) -> Self {
        match value {
            0x0000_0000 => Command::Ping,
            0x0000_0001 => Command::ReadRom,
            _ => Command::Unknown,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum Response {
    Pong = 0x0000_0000,
    RomMetadata = 0x0000_0001,
    Error = 0x8000_0000,
    NoRom = 0x8000_0001,
}

impl Response {
    pub const fn size() -> usize {
        core::mem::size_of::<Self>()
    }

    pub fn to_bytes(&self, buf: &mut [u8]) {
        let value = *self as u32;
        buf[..4].copy_from_slice(&value.to_le_bytes());
    }
}
