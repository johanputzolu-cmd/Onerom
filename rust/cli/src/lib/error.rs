// Copyright (C) 2026 Piers Finlayson <piers@piers.rocks>
//
// MIT License

//! Shared error type for the One ROM CLI library.

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("USB error: {0}")]
    Usb(String),

    #[error("No One ROM devices found")]
    NoDevices,

    #[error("Multiple devices found - use --device to select one.\n  Found: {}", .0.join(", "))]
    MultipleDevices(Vec<String>),

    #[error("Device not found: {0}")]
    DeviceNotFound(String),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("{0}")]
    Other(String),

    #[error("Unknown board type: {0}\nBoard types: {1}")]
    InvalidBoard(String, String),

    #[error("Cannot specify both --device and --board together")]
    DeviceAndBoard,

    #[error("Operation does not apply to --device")]
    Device,

    #[error("No --device was specified")]
    NoDevice,

    #[error("Command '{0}' has not been implemented")]
    Unimplemented(String),

    #[error(
        "The operation attempted to access an unsupported memory region: address {0:#010x} length {1:#010x}"
    )]
    InvalidMemoryRange(u32, u32),

    #[error("The specified memory range is not accessible when One ROM isn't running")]
    MemoryDeviceNotRunning,

    #[error("The specificied memory range is not writeable")]
    MemoryNotWriteable,

    #[error("This operation can only be performed on a One ROM that is running")]
    NotRunning,

    #[error("This operation cannot be performed as the ROM type is unknown")]
    UnknownRomType,

    #[error(
        "The operation attempted to access past the end of a live ROM image\n  ROM type {0} image size is {1} bytes"
    )]
    LiveOutOfBounds(String, usize),
}

impl From<onerom_fw::Error> for Error {
    fn from(e: onerom_fw::Error) -> Self {
        Self::Other(e.to_string())
    }
}
