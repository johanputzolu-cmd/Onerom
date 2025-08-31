//! One ROM Lab firmware

// Copyright (c) 2025 Piers Finlayson <piers@piers.rocks>
//
// MIT licence

use airfrog_rpc::target::RawMemoryRegion;
use airfrog_rpc::target::Target;
use alloc::vec::Vec;
#[allow(unused_imports)]
use defmt::{debug, error, info, trace, warn};
use embassy_time::Timer;
use onerom_lab::rpc::{Command, Response};

use crate::Rom;
use crate::info::LAB_RAM_INFO;

// Type used to hide RPC type complexity
#[cfg(feature = "control")]
type Rpc = Target<RawMemoryRegion, RawMemoryRegion>;

// Buffers used as command channels for Airfrog comms
const RPC_CHANNEL_SIZE: usize = 512;
static mut RPC_CMD_CHANNEL: [u8; RPC_CHANNEL_SIZE] = [0; RPC_CHANNEL_SIZE];
static mut RPC_RSP_CHANNEL: [u8; RPC_CHANNEL_SIZE] = [0; RPC_CHANNEL_SIZE];

pub(crate) struct Control {
    rpc: Rpc,
    rom: Rom,
}

impl Control {
    /// Create the Control object.
    pub fn new(rom: Rom) -> Self {
        // Create RPC channels used to control One ROM Lab.  This involves
        // unsafe code, as we have to get a pointer to statically allocated
        // buffers and pass them to the channel.
        #[allow(static_mut_refs)]
        let rpc = unsafe {
            LAB_RAM_INFO.rpc_cmd_channel = &RPC_CMD_CHANNEL as *const _ as *const core::ffi::c_void;
            LAB_RAM_INFO.rpc_rsp_channel = &RPC_RSP_CHANNEL as *const _ as *const core::ffi::c_void;
            let rpc_cmd_channel_ptr = &raw mut RPC_CMD_CHANNEL as u8 as *mut u8;
            let rpc_rsp_channel_ptr = &raw mut RPC_RSP_CHANNEL as u8 as *mut u8;
            let cmd_memory = RawMemoryRegion::new(rpc_cmd_channel_ptr, RPC_CHANNEL_SIZE);
            let rsp_memory = RawMemoryRegion::new(rpc_rsp_channel_ptr, RPC_CHANNEL_SIZE);
            Target::new(cmd_memory, rsp_memory).expect("Failed to create RPC Channels")
        };

        Self { rpc, rom }
    }

    /// Run the Control handler.  Call from within a task.
    pub async fn run(&mut self) -> ! {
        loop {
            // Wait for a command.  Target only returns errors if it can't read
            // RAM - so we're OK to expect().
            let cmd_size = loop {
                match self
                    .rpc
                    .command_size()
                    .expect("RPC Channel error getting command size")
                {
                    Some(size) => {
                        if size >= core::mem::size_of::<Command>() {
                            break size;
                        } else {
                            warn!("Received under-sized command, ignoring");
                            ()
                        }
                    }
                    None => (),
                }
                Timer::after_millis(1).await;
            };

            // Read the command from the channel
            assert!(cmd_size > 0);
            let mut cmd_data = Vec::with_capacity(cmd_size);
            let received = self
                .rpc
                .read_command(&mut cmd_data)
                .expect("RPC Channel error reading command");
            assert!(received == cmd_size);

            // Convert the command bytes into a Command enum
            let command_u32 = u32::from_le_bytes(cmd_data[..4].try_into().unwrap());
            let command = Command::from(command_u32);

            // Handle the command including sending a response
            self.handle_command(command, &cmd_data).await;

            // Mark the original command as processed (done after any response is
            // sent).
            self.rpc
                .mark_command_processed()
                .expect("RPC Channel error marking command processed");
        }
    }

    async fn handle_command(&mut self, command: Command, cmd_data: &[u8]) {
        match command {
            Command::Ping => {
                self.send_response_no_data(Response::Pong);
            }
            Command::ReadRom => {
                // Read the ROM
                if let Some(rom) = self.rom.read_rom().await {
                    // Found a ROM - build the response from the metadata
                    let size = rom.metadata_size();
                    let mut buf = Vec::with_capacity(size + Response::size());
                    Response::RomMetadata.to_bytes(&mut buf);
                    rom.metadata_bytes(&mut buf);

                    // Send it
                    self.send_response_data(&buf);
                } else {
                    self.send_response_no_data(Response::NoRom);
                }
            }
            Command::Unknown => {
                // Ignore
                warn!(
                    "Received unknown command 0x{:02x}{:02x}{:02x}{:02x}, ignoring",
                    cmd_data[0], cmd_data[1], cmd_data[2], cmd_data[3]
                );
            }
        }
    }

    #[cfg(feature = "control")]
    fn send_response_no_data(&mut self, response: Response) {
        let mut buf = [0u8; Response::size()];
        response.to_bytes(&mut buf);
        self.rpc
            .send_response(&buf)
            .expect("RPC Channel error sending response (no data)");
    }

    fn send_response_data(&mut self, data: &[u8]) {
        self.rpc
            .send_response(data)
            .expect("RPC Channel error sending response (with data)");
    }
}
