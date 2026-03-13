// Copyright (C) 2026 Piers Finlayson <piers@piers.rocks>
//
// MIT License

//! Shared error type for the One ROM CLI library.

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("USB error: {0}")]
    Usb(String),

    #[error("No One ROMs found")]
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

    #[error("Could not determine board type from connected device")]
    NoBoardFromDevice,

    #[error("Either --board or --device must be specified")]
    NoBoardOrDevice,

    #[error("Version '{0}' not found. Available releases: {1}")]
    VersionNotFound(String, String),

    #[error("No latest release found in manifest")]
    NoLatestRelease,

    #[error("License not accepted")]
    LicenseNotAccepted,

    #[error("Firmware image supplied is larger than the maximum supported: {0} bytes vs {1} bytes")]
    FirmwareTooLarge(usize, usize),

    #[error("Assembled firmware has parse errors (use --force to override):\n{0}")]
    FirmwareValidation(String),

    #[error(
        "--base-firmware without --config or --rom requires --no-config to confirm flashing firmware with no ROM images"
    )]
    NoConfigNotConfirmed,

    #[error("Failed to stop device, cannot proceed")]
    DeviceRunning,

    #[error("Flash verification failed at offset {0:#010x}: expected {1:#04x}, got {2:#04x}")]
    VerifyFailed(usize, u8, u8),

    #[error("Invalid argument: {0}")]
    InvalidArgument(String),

    #[error("No firmware source specified. Use --config, --rom, --firmware, or --base-firmware.")]
    NoFirmwareSource,
}

impl From<onerom_fw::Error> for Error {
    fn from(e: onerom_fw::Error) -> Self {
        Self::Other(e.to_string())
    }
}

impl From<onerom_config::Error> for Error {
    fn from(e: onerom_config::Error) -> Self {
        Self::Other(format!("{e}"))
    }
}
