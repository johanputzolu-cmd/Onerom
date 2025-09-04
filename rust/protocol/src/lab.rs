//! One ROM Protocol - One ROM Lab support
//! 
//! Used by both the One ROM Lab firmware and host tools to communicate.
//! 
//! Uses `airfrog-rpc` for the underlying RPC transport.
//! 
//! The host can retrieve the RAM metada using `sdrr-fw-parser` which provides
//! the RAM channel addresses required for RPC communication.
//! 
//! See `airfrog::firmware::onerom_lab` for example host usage.

// Copyright (c) 2025 Piers Finlayson <piers@piers.rocks>
//
// MIT licence

/// Commands supported by One ROM Lab
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum Command {
    /// Response with a Pong
    Ping = 0x0000_0000,

    /// Trigger a read of the connected ROM
    ReadRom = 0x0000_0001,

    /// Unknown command, do not use
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

impl From<Command> for u32 {
    fn from(cmd: Command) -> Self {
        cmd as u32
    }
}

impl Command {
    pub fn size() -> usize {
        core::mem::size_of::<Self>()
    }

    pub fn as_bytes(&self) -> [u8; 4] {
        (*self as u32).to_le_bytes()
    }
}

/// Responses from One ROM Lab to Commands
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum Response {
    /// Ping response
    Pong = 0x0000_0000,

    /// ReadRom successful response.  Following this word, is the ROM metadata
    /// as a sequence of bytes:
    /// - Name of the ROM, followed by 0
    /// - Part number of the ROM, followed by 0
    /// - 32-bit wrapping checksum of the ROM, little endian encoded
    /// - 20 byte SHA1 digest of the ROM
    RomMetadata = 0x0000_0001,

    /// One ROM Lab hit an error
    Error = 0x8000_0000,

    /// One ROM Lab did not detect a ROM connected, but it may have been
    /// unrecognised
    NoRom = 0x8000_0001,

    Unknown = 0xFFFF_FFFF,
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

impl From<u32> for Response {
    fn from(value: u32) -> Self {
        match value {
            0x0000_0000 => Response::Pong,
            0x0000_0001 => Response::RomMetadata,
            0x8000_0000 => Response::Error,
            0x8000_0001 => Response::NoRom,
            _ => Response::Unknown,
        }
    }
}