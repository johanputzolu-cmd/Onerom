// Copyright (C) 2025 Piers Finlayson <piers@piers.rocks>
//
// MIT License

// Copyright (C) 2025 Piers Finlayson <piers@piers.rocks>
//
// MIT License

//! Tests for onerom-gen Builder
//!
//! Progressive validation of metadata and ROM image generation.
//!
//! # Test Plan
//!
//! ## Phase 1: Basic Structure Tests ✓ COMPLETE
//! - [x] Single ROM set, single ROM, no boot logging
//! - [x] Validate metadata header (magic, version, count)
//! - [x] Validate ROM set structure (data ptr, size, roms ptr, count, serve alg, multi-cs)
//! - [x] Validate ROM pointer array
//! - [x] Validate ROM info structure (rom type, cs1/cs2/cs3 states)
//! - [x] Validate pointer chain (header → rom set → rom array → rom info)
//!
//! ## Phase 2: Multiple ROM Sets ✓ COMPLETE
//! - [x] Multiple single ROM sets (2-3 sets)
//! - [x] Validate ROM set array is correct
//! - [x] Validate each set independently
//! - [x] Validate each ROM info independently
//!
//! ## Phase 3: CS Configuration Tests
//! - [ ] 2332 with CS1 + CS2 (both active low)
//! - [ ] 2332 with CS1 active low, CS2 active high
//! - [ ] 2316 with CS1 + CS2 + CS3 (all active low)
//! - [ ] 2316 with mixed active high/low states
//! - [ ] Validate CS states stored correctly
//!
//! ## Phase 4: Boot Logging (Filenames)
//! - [x] Single ROM with boot_logging enabled
//! - [x] Validate ROM info structure is 8 bytes (not 4)
//! - [x] Validate filename pointer points within metadata
//! - [x] Validate null-terminated filename string
//! - [ ] Multiple ROMs with boot_logging
//!
//! ## Phase 5: Size Handling
//! - [ ] Exact size match (no size_handling needed)
//! - [ ] Duplicate (smaller file, exact divisor)
//! - [ ] Pad (smaller file)
//! - [ ] Error cases (too large, wrong divisor, unnecessary size_handling)
//!
//! ## Phase 6: Multi-ROM Sets
//! - [ ] Banked ROM sets
//! - [ ] Multi ROM sets
//! - [ ] Validate serve algorithm selection
//! - [ ] Validate multi-CS state
//!
//! ## Phase 7: ROM Images Buffer
//! - [ ] Validate buffer size matches expectations
//! - [ ] Note: ROM image bytes are "mangled" (address/data transformations)
//! - [ ] Use demangling functions from other crate to verify correctness
//! - [ ] Test address mapping
//! - [ ] Test data bit reordering
//!
//! ## Phase 8: Edge Cases
//! - [ ] Maximum ROM sets
//! - [ ] Minimum ROM sizes (2KB)
//! - [ ] Maximum ROM sizes (64KB)
//! - [ ] Missing CS config (should error)
//! - [ ] Wrong size_handling for exact match (should error)
//! - [ ] Adding files out of order
//! - [ ] Adding duplicate files (should error)
//! - [ ] Missing files at build time (should error)

#[cfg(test)]
mod tests {
    use onerom_config::fw::{FirmwareProperties, FirmwareVersion, ServeAlg};
    use onerom_config::hw::Board;
    use onerom_gen::builder::{Builder, FileData};
    use onerom_gen::image::CsLogic;

    // ========================================================================
    // Constants from C headers
    // ========================================================================

    const HEADER_MAGIC: &[u8; 16] = b"ONEROM_METADATA\0";
    const HEADER_VERSION: u32 = 1;
    const METADATA_HEADER_LEN: usize = 256;
    const ROM_SET_METADATA_LEN: usize = 16;
    const ROM_INFO_METADATA_LEN: usize = 4;
    const ROM_INFO_METADATA_LEN_WITH_FILENAME: usize = 8;

    // ROM type C enum values (from Rom::rom_type_c_enum_val in image.rs)
    const ROM_TYPE_2316: u8 = 0;
    const ROM_TYPE_2332: u8 = 1;
    const ROM_TYPE_2364: u8 = 2;

    // ========================================================================
    // Helper: Parse Metadata Header
    // ========================================================================

    /// Represents the onerom_metadata_header_t C structure
    #[derive(Debug)]
    struct MetadataHeader {
        magic: [u8; 16],
        version: u32,
        rom_set_count: u8,
        rom_sets_ptr: u32,
    }

    impl MetadataHeader {
        /// Parse the metadata header from the start of the buffer
        fn parse(buf: &[u8]) -> Self {
            assert!(
                buf.len() >= METADATA_HEADER_LEN,
                "Buffer too small: {} bytes, need {}",
                buf.len(),
                METADATA_HEADER_LEN
            );

            // Magic: offset 0, 16 bytes
            let mut magic = [0u8; 16];
            magic.copy_from_slice(&buf[0..16]);

            // Version: offset 16, 4 bytes (u32 little-endian)
            let version = u32::from_le_bytes([buf[16], buf[17], buf[18], buf[19]]);

            // ROM set count: offset 20, 1 byte
            let rom_set_count = buf[20];

            // Padding: offset 21, 3 bytes (we skip these)

            // ROM sets pointer: offset 24, 4 bytes (u32 little-endian)
            let rom_sets_ptr = u32::from_le_bytes([buf[24], buf[25], buf[26], buf[27]]);

            Self {
                magic,
                version,
                rom_set_count,
                rom_sets_ptr,
            }
        }

        /// Validate the header has correct magic and version
        fn validate_basic(&self) {
            assert_eq!(
                &self.magic, HEADER_MAGIC,
                "Magic bytes mismatch. Expected {:?}, got {:?}",
                HEADER_MAGIC, &self.magic
            );

            assert_eq!(
                self.version, HEADER_VERSION,
                "Version mismatch. Expected {}, got {}",
                HEADER_VERSION, self.version
            );

            assert!(
                self.rom_set_count > 0,
                "ROM set count must be > 0, got {}",
                self.rom_set_count
            );
        }
    }

    // ========================================================================
    // Helper: Parse ROM Set Structure
    // ========================================================================

    /// Represents the sdrr_rom_set_t C structure
    #[derive(Debug)]
    struct RomSetStruct {
        data_ptr: u32,
        size: u32,
        roms_ptr: u32,
        rom_count: u8,
        serve_alg: u8,
        multi_cs_state: u8,
    }

    impl RomSetStruct {
        /// Parse ROM set structure from buffer at given offset
        fn parse(buf: &[u8], offset: usize) -> Self {
            assert!(
                buf.len() >= offset + ROM_SET_METADATA_LEN,
                "Buffer too small: {} bytes, need {} at offset {}",
                buf.len(),
                offset + ROM_SET_METADATA_LEN,
                offset
            );

            // Data pointer: offset + 0, 4 bytes
            let data_ptr = u32::from_le_bytes([
                buf[offset],
                buf[offset + 1],
                buf[offset + 2],
                buf[offset + 3],
            ]);

            // Size: offset + 4, 4 bytes
            let size = u32::from_le_bytes([
                buf[offset + 4],
                buf[offset + 5],
                buf[offset + 6],
                buf[offset + 7],
            ]);

            // ROMs pointer: offset + 8, 4 bytes
            let roms_ptr = u32::from_le_bytes([
                buf[offset + 8],
                buf[offset + 9],
                buf[offset + 10],
                buf[offset + 11],
            ]);

            // ROM count: offset + 12, 1 byte
            let rom_count = buf[offset + 12];

            // Serve algorithm: offset + 13, 1 byte
            let serve_alg = buf[offset + 13];

            // Multi-CS state: offset + 14, 1 byte
            let multi_cs_state = buf[offset + 14];

            // Padding at offset + 15 (1 byte) - ignored

            Self {
                data_ptr,
                size,
                roms_ptr,
                rom_count,
                serve_alg,
                multi_cs_state,
            }
        }
    }

    // ========================================================================
    // Helper: Parse ROM Info Structure
    // ========================================================================

    /// Represents the sdrr_rom_info_t C structure
    #[derive(Debug)]
    struct RomInfoStruct {
        rom_type: u8,
        cs1_state: u8,
        cs2_state: u8,
        cs3_state: u8,
        filename_ptr: Option<u32>,
    }

    impl RomInfoStruct {
        /// Parse ROM info structure from buffer at given offset (without filename)
        fn parse(buf: &[u8], offset: usize) -> Self {
            assert!(
                buf.len() >= offset + ROM_INFO_METADATA_LEN,
                "Buffer too small: {} bytes, need {} at offset {}",
                buf.len(),
                offset + ROM_INFO_METADATA_LEN,
                offset
            );

            let rom_type = buf[offset];
            let cs1_state = buf[offset + 1];
            let cs2_state = buf[offset + 2];
            let cs3_state = buf[offset + 3];

            Self {
                rom_type,
                cs1_state,
                cs2_state,
                cs3_state,
                filename_ptr: None,
            }
        }

        /// Parse ROM info structure from buffer at given offset (with filename)
        fn parse_with_filename(buf: &[u8], offset: usize) -> Self {
            assert!(
                buf.len() >= offset + ROM_INFO_METADATA_LEN_WITH_FILENAME,
                "Buffer too small: {} bytes, need {} at offset {}",
                buf.len(),
                offset + ROM_INFO_METADATA_LEN_WITH_FILENAME,
                offset
            );

            let rom_type = buf[offset];
            let cs1_state = buf[offset + 1];
            let cs2_state = buf[offset + 2];
            let cs3_state = buf[offset + 3];

            let filename_ptr = u32::from_le_bytes([
                buf[offset + 4],
                buf[offset + 5],
                buf[offset + 6],
                buf[offset + 7],
            ]);

            Self {
                rom_type,
                cs1_state,
                cs2_state,
                cs3_state,
                filename_ptr: Some(filename_ptr),
            }
        }
    }

    // ========================================================================
    // Helper: Create test firmware properties
    // ========================================================================

    fn default_fw_props() -> FirmwareProperties {
        FirmwareProperties::new(
            FirmwareVersion::new(0, 5, 1, 0),
            Board::Ice24UsbH,
            ServeAlg::Default,
            false, // boot_logging disabled
        )
    }

    fn fw_props_with_logging() -> FirmwareProperties {
        FirmwareProperties::new(
            FirmwareVersion::new(0, 5, 1, 0),
            Board::Ice24UsbH,
            ServeAlg::Default,
            true, // boot_logging enabled
        )
    }

    // ========================================================================
    // Helper: Parse null-terminated string
    // ========================================================================

    fn parse_null_terminated_string(buf: &[u8], offset: usize) -> String {
        let mut end = offset;
        while end < buf.len() && buf[end] != 0 {
            end += 1;
        }

        assert!(
            end < buf.len(),
            "No null terminator found starting at offset {}",
            offset
        );

        String::from_utf8_lossy(&buf[offset..end]).to_string()
    }

    // ========================================================================
    // Helper: Create test ROM data
    // ========================================================================

    fn create_test_rom_data(size: usize, fill_byte: u8) -> Vec<u8> {
        vec![fill_byte; size]
    }

    // ========================================================================
    // TEST 1: Simplest possible - single ROM set, single ROM
    // ========================================================================

    #[test]
    fn test_phase1_single_rom_basic() {
        // Minimal JSON config: single ROM set with one 2364 ROM (8KB)
        let json = r#"{
            "version": 1,
            "description": "Phase 1 basic test",
            "rom_sets": [{
                "type": "single",
                "roms": [{
                    "file": "test.rom",
                    "type": "2364",
                    "cs1": "active_low"
                }]
            }]
        }"#;

        // Parse the JSON
        let mut builder = Builder::from_json(json).expect("Failed to parse JSON");

        // Get the file specs - should be exactly 1
        let file_specs = builder.file_specs();
        assert_eq!(file_specs.len(), 1, "Should have exactly 1 file");
        assert_eq!(file_specs[0].id, 0, "File ID should be 0");
        assert_eq!(file_specs[0].source, "test.rom", "File source should match");

        // Create 8KB of test data (2364 is 8KB)
        let rom_data = create_test_rom_data(8192, 0xAA);

        // Add the file
        builder
            .add_file(FileData {
                id: 0,
                data: rom_data,
            })
            .expect("Failed to add file");

        // Build the metadata and ROM images
        let props = default_fw_props();
        let (metadata_buf, rom_images_buf) = builder.build(props).expect("Build failed");

        // Basic sanity checks
        assert!(
            !metadata_buf.is_empty(),
            "Metadata buffer should not be empty"
        );
        assert!(
            metadata_buf.len() >= METADATA_HEADER_LEN,
            "Metadata buffer should be at least {} bytes, got {}",
            METADATA_HEADER_LEN,
            metadata_buf.len()
        );
        assert!(
            !rom_images_buf.is_empty(),
            "ROM images buffer should not be empty"
        );

        // Parse and validate the metadata header
        let header = MetadataHeader::parse(&metadata_buf);
        header.validate_basic();

        // Check ROM set count
        assert_eq!(header.rom_set_count, 1, "Should have exactly 1 ROM set");

        println!("✓ Phase 1 Test 1: Basic single ROM set passed");
        println!("  - Metadata size: {} bytes", metadata_buf.len());
        println!("  - ROM images size: {} bytes", rom_images_buf.len());
        println!("  - ROM set count: {}", header.rom_set_count);
    }

    // ========================================================================
    // TEST 2: Validate ROM Set Structure
    // ========================================================================

    #[test]
    fn test_phase1_rom_set_structure() {
        let json = r#"{
            "version": 1,
            "description": "Phase 1 ROM set structure test",
            "rom_sets": [{
                "type": "single",
                "roms": [{
                    "file": "test.rom",
                    "type": "2364",
                    "cs1": "active_low"
                }]
            }]
        }"#;

        let mut builder = Builder::from_json(json).expect("Failed to parse JSON");
        let rom_data = create_test_rom_data(8192, 0xAA);
        builder
            .add_file(FileData {
                id: 0,
                data: rom_data,
            })
            .expect("Failed to add file");

        let props = default_fw_props();
        let board = props.board();
        let (metadata_buf, _rom_images_buf) = builder.build(props).expect("Build failed");

        // Parse metadata header
        let header = MetadataHeader::parse(&metadata_buf);
        header.validate_basic();

        // Calculate where ROM set structure should be
        // rom_sets_ptr is an absolute flash address, need to convert to metadata buffer offset
        let flash_base = board.mcu_family().get_flash_base();
        let rom_set_offset = (header.rom_sets_ptr - flash_base) as usize;

        // Validate offset is within metadata buffer
        assert!(
            rom_set_offset < metadata_buf.len(),
            "ROM set pointer {} (offset {}) outside metadata buffer (size {})",
            header.rom_sets_ptr,
            rom_set_offset,
            metadata_buf.len()
        );

        // Parse the ROM set structure
        let rom_set = RomSetStruct::parse(&metadata_buf, rom_set_offset);

        // Validate data pointer (should be flash_base + 64KB)
        let expected_data_ptr = flash_base + 65536;
        assert_eq!(
            rom_set.data_ptr, expected_data_ptr,
            "Data pointer mismatch. Expected 0x{:08X}, got 0x{:08X}",
            expected_data_ptr, rom_set.data_ptr
        );

        // Validate size (STM32F4 single ROM = 16KB)
        let expected_size = 16384u32;
        assert_eq!(
            rom_set.size, expected_size,
            "Size mismatch. Expected {} bytes, got {} bytes",
            expected_size, rom_set.size
        );

        // Validate ROMs pointer is within metadata
        let roms_ptr_offset = (rom_set.roms_ptr - flash_base) as usize;
        assert!(
            roms_ptr_offset < metadata_buf.len(),
            "ROMs pointer {} (offset {}) outside metadata buffer (size {})",
            rom_set.roms_ptr,
            roms_ptr_offset,
            metadata_buf.len()
        );

        // Validate ROM count
        assert_eq!(
            rom_set.rom_count, 1,
            "ROM count should be 1, got {}",
            rom_set.rom_count
        );

        // Validate serve algorithm (single ROM uses AddrOnCs)
        let expected_serve_alg = ServeAlg::AddrOnCs.c_enum_value();
        assert_eq!(
            rom_set.serve_alg, expected_serve_alg,
            "Serve algorithm mismatch. Expected {} (AddrOnCs), got {}",
            expected_serve_alg, rom_set.serve_alg
        );

        // Validate multi-CS state (single ROM should be Ignore)
        let expected_multi_cs = CsLogic::Ignore.c_enum_val();
        assert_eq!(
            rom_set.multi_cs_state, expected_multi_cs,
            "Multi-CS state mismatch. Expected {} (Ignore), got {}",
            expected_multi_cs, rom_set.multi_cs_state
        );

        println!("✓ Phase 1 Test 2: ROM set structure validation passed");
        println!("  - Data pointer: 0x{:08X}", rom_set.data_ptr);
        println!("  - Size: {} bytes", rom_set.size);
        println!("  - ROMs pointer: 0x{:08X}", rom_set.roms_ptr);
        println!("  - ROM count: {}", rom_set.rom_count);
        println!("  - Serve algorithm: {}", rom_set.serve_alg);
        println!("  - Multi-CS state: {}", rom_set.multi_cs_state);
    }

    // ========================================================================
    // TEST 3: Validate ROM Info Structure
    // ========================================================================

    #[test]
    fn test_phase1_rom_info_structure() {
        let json = r#"{
            "version": 1,
            "description": "Phase 1 ROM info structure test",
            "rom_sets": [{
                "type": "single",
                "roms": [{
                    "file": "test.rom",
                    "type": "2364",
                    "cs1": "active_low"
                }]
            }]
        }"#;

        let mut builder = Builder::from_json(json).expect("Failed to parse JSON");
        let rom_data = create_test_rom_data(8192, 0xAA);
        builder
            .add_file(FileData {
                id: 0,
                data: rom_data,
            })
            .expect("Failed to add file");

        let props = default_fw_props();
        let board = props.board();
        let flash_base = board.mcu_family().get_flash_base();
        let (metadata_buf, _rom_images_buf) = builder.build(props).expect("Build failed");

        // Parse metadata header and ROM set
        let header = MetadataHeader::parse(&metadata_buf);
        let rom_set_offset = (header.rom_sets_ptr - flash_base) as usize;
        let rom_set = RomSetStruct::parse(&metadata_buf, rom_set_offset);

        // Parse ROM pointer array to get pointer to first ROM info
        let rom_array_offset = (rom_set.roms_ptr - flash_base) as usize;
        assert!(
            rom_array_offset + 4 <= metadata_buf.len(),
            "ROM array pointer {} (offset {}) outside metadata buffer",
            rom_set.roms_ptr,
            rom_array_offset
        );

        // Read the first pointer from the ROM pointer array (4 bytes)
        let rom_info_ptr = u32::from_le_bytes([
            metadata_buf[rom_array_offset],
            metadata_buf[rom_array_offset + 1],
            metadata_buf[rom_array_offset + 2],
            metadata_buf[rom_array_offset + 3],
        ]);

        // Convert to buffer offset
        let rom_info_offset = (rom_info_ptr - flash_base) as usize;
        assert!(
            rom_info_offset < metadata_buf.len(),
            "ROM info pointer {} (offset {}) outside metadata buffer",
            rom_info_ptr,
            rom_info_offset
        );

        // Parse the ROM info structure
        let rom_info = RomInfoStruct::parse(&metadata_buf, rom_info_offset);

        // Validate ROM type (2364 = 2)
        assert_eq!(
            rom_info.rom_type, ROM_TYPE_2364,
            "ROM type mismatch. Expected {} (2364), got {}",
            ROM_TYPE_2364, rom_info.rom_type
        );

        // Validate CS1 state (active_low = 0)
        let expected_cs1 = CsLogic::ActiveLow.c_enum_val();
        assert_eq!(
            rom_info.cs1_state, expected_cs1,
            "CS1 state mismatch. Expected {} (ActiveLow), got {}",
            expected_cs1, rom_info.cs1_state
        );

        // Validate CS2 state (not used for 2364, should be CS_NOT_USED = 2)
        let expected_cs2 = CsLogic::Ignore.c_enum_val();
        assert_eq!(
            rom_info.cs2_state, expected_cs2,
            "CS2 state mismatch. Expected {} (Ignore), got {}",
            expected_cs2, rom_info.cs2_state
        );

        // Validate CS3 state (not used for 2364, should be CS_NOT_USED = 2)
        let expected_cs3 = CsLogic::Ignore.c_enum_val();
        assert_eq!(
            rom_info.cs3_state, expected_cs3,
            "CS3 state mismatch. Expected {} (Ignore), got {}",
            expected_cs3, rom_info.cs3_state
        );

        println!("✓ Phase 1 Test 3: ROM info structure validation passed");
        println!("  - ROM type: {} (2364)", rom_info.rom_type);
        println!("  - CS1 state: {} (ActiveLow)", rom_info.cs1_state);
        println!("  - CS2 state: {} (Ignore)", rom_info.cs2_state);
        println!("  - CS3 state: {} (Ignore)", rom_info.cs3_state);
    }

    // ========================================================================
    // PHASE 2: Multiple ROM Sets
    // ========================================================================

    // ========================================================================
    // TEST 5: Two ROM Sets
    // ========================================================================

    #[test]
    fn test_phase2_two_rom_sets() {
        let json = r#"{
            "version": 1,
            "description": "Phase 2 two ROM sets test",
            "rom_sets": [
                {
                    "type": "single",
                    "description": "Set 0 - 2364",
                    "roms": [{
                        "file": "set0.rom",
                        "type": "2364",
                        "cs1": "active_low"
                    }]
                },
                {
                    "type": "single",
                    "description": "Set 1 - 2332",
                    "roms": [{
                        "file": "set1.rom",
                        "type": "2332",
                        "cs1": "active_low",
                        "cs2": "active_high"
                    }]
                }
            ]
        }"#;

        let mut builder = Builder::from_json(json).expect("Failed to parse JSON");

        // Add ROM data for both sets
        builder
            .add_file(FileData {
                id: 0,
                data: create_test_rom_data(8192, 0xAA), // 2364 = 8KB
            })
            .expect("Failed to add file 0");

        builder
            .add_file(FileData {
                id: 1,
                data: create_test_rom_data(4096, 0x55), // 2332 = 4KB
            })
            .expect("Failed to add file 1");

        let props = default_fw_props();
        let board = props.board();
        let flash_base = board.mcu_family().get_flash_base();
        let (metadata_buf, _rom_images_buf) = builder.build(props).expect("Build failed");

        // Parse metadata header
        let header = MetadataHeader::parse(&metadata_buf);
        header.validate_basic();

        // Validate we have 2 ROM sets
        assert_eq!(
            header.rom_set_count, 2,
            "Should have 2 ROM sets, got {}",
            header.rom_set_count
        );

        // Parse both ROM sets
        let rom_set0_offset = (header.rom_sets_ptr - flash_base) as usize;
        let rom_set0 = RomSetStruct::parse(&metadata_buf, rom_set0_offset);

        let rom_set1_offset = rom_set0_offset + ROM_SET_METADATA_LEN;
        let rom_set1 = RomSetStruct::parse(&metadata_buf, rom_set1_offset);

        // Validate Set 0 (2364)
        assert_eq!(rom_set0.rom_count, 1, "Set 0 should have 1 ROM");
        assert_eq!(rom_set0.size, 16384, "Set 0 size should be 16KB");
        assert_eq!(
            rom_set0.serve_alg,
            ServeAlg::AddrOnCs.c_enum_value(),
            "Set 0 serve algorithm mismatch"
        );

        // Validate Set 1 (2332)
        assert_eq!(rom_set1.rom_count, 1, "Set 1 should have 1 ROM");
        assert_eq!(rom_set1.size, 16384, "Set 1 size should be 16KB");
        assert_eq!(
            rom_set1.serve_alg,
            ServeAlg::AddrOnCs.c_enum_value(),
            "Set 1 serve algorithm mismatch"
        );

        // Validate Set 0 data pointer (flash_base + 64KB)
        let expected_data_ptr0 = flash_base + 65536;
        assert_eq!(
            rom_set0.data_ptr, expected_data_ptr0,
            "Set 0 data pointer mismatch"
        );

        // Validate Set 1 data pointer (flash_base + 64KB + 16KB)
        let expected_data_ptr1 = flash_base + 65536 + 16384;
        assert_eq!(
            rom_set1.data_ptr, expected_data_ptr1,
            "Set 1 data pointer mismatch"
        );

        // Parse ROM info for Set 0
        let rom_array0_offset = (rom_set0.roms_ptr - flash_base) as usize;
        let rom_info0_ptr = u32::from_le_bytes([
            metadata_buf[rom_array0_offset],
            metadata_buf[rom_array0_offset + 1],
            metadata_buf[rom_array0_offset + 2],
            metadata_buf[rom_array0_offset + 3],
        ]);
        let rom_info0_offset = (rom_info0_ptr - flash_base) as usize;
        let rom_info0 = RomInfoStruct::parse(&metadata_buf, rom_info0_offset);

        // Validate Set 0 ROM info
        assert_eq!(rom_info0.rom_type, ROM_TYPE_2364, "Set 0 ROM type mismatch");
        assert_eq!(rom_info0.cs1_state, CsLogic::ActiveLow.c_enum_val());
        assert_eq!(rom_info0.cs2_state, CsLogic::Ignore.c_enum_val());
        assert_eq!(rom_info0.cs3_state, CsLogic::Ignore.c_enum_val());

        // Parse ROM info for Set 1
        let rom_array1_offset = (rom_set1.roms_ptr - flash_base) as usize;
        let rom_info1_ptr = u32::from_le_bytes([
            metadata_buf[rom_array1_offset],
            metadata_buf[rom_array1_offset + 1],
            metadata_buf[rom_array1_offset + 2],
            metadata_buf[rom_array1_offset + 3],
        ]);
        let rom_info1_offset = (rom_info1_ptr - flash_base) as usize;
        let rom_info1 = RomInfoStruct::parse(&metadata_buf, rom_info1_offset);

        // Validate Set 1 ROM info
        assert_eq!(rom_info1.rom_type, ROM_TYPE_2332, "Set 1 ROM type mismatch");
        assert_eq!(rom_info1.cs1_state, CsLogic::ActiveLow.c_enum_val());
        assert_eq!(rom_info1.cs2_state, CsLogic::ActiveHigh.c_enum_val());
        assert_eq!(rom_info1.cs3_state, CsLogic::Ignore.c_enum_val());

        println!("✓ Phase 2 Test 1: Two ROM sets validation passed");
        println!("  Set 0:");
        println!("    - ROM type: {} (2364)", rom_info0.rom_type);
        println!("    - Data pointer: 0x{:08X}", rom_set0.data_ptr);
        println!("    - Size: {} bytes", rom_set0.size);
        println!("    - CS1: {} (ActiveLow)", rom_info0.cs1_state);
        println!("  Set 1:");
        println!("    - ROM type: {} (2332)", rom_info1.rom_type);
        println!("    - Data pointer: 0x{:08X}", rom_set1.data_ptr);
        println!("    - Size: {} bytes", rom_set1.size);
        println!(
            "    - CS1: {} (ActiveLow), CS2: {} (ActiveHigh)",
            rom_info1.cs1_state, rom_info1.cs2_state
        );
    }

    // ========================================================================
    // TEST 6: Three ROM Sets
    // ========================================================================

    #[test]
    fn test_phase2_three_rom_sets() {
        let json = r#"{
            "version": 1,
            "description": "Phase 2 three ROM sets test",
            "rom_sets": [
                {
                    "type": "single",
                    "roms": [{
                        "file": "set0.rom",
                        "type": "2364",
                        "cs1": "active_low"
                    }]
                },
                {
                    "type": "single",
                    "roms": [{
                        "file": "set1.rom",
                        "type": "2332",
                        "cs1": "active_low",
                        "cs2": "active_high"
                    }]
                },
                {
                    "type": "single",
                    "roms": [{
                        "file": "set2.rom",
                        "type": "2316",
                        "cs1": "active_low",
                        "cs2": "active_low",
                        "cs3": "active_low"
                    }]
                }
            ]
        }"#;

        let mut builder = Builder::from_json(json).expect("Failed to parse JSON");

        // Add ROM data for all three sets
        builder
            .add_file(FileData {
                id: 0,
                data: create_test_rom_data(8192, 0xAA), // 2364 = 8KB
            })
            .expect("Failed to add file 0");

        builder
            .add_file(FileData {
                id: 1,
                data: create_test_rom_data(4096, 0x55), // 2332 = 4KB
            })
            .expect("Failed to add file 1");

        builder
            .add_file(FileData {
                id: 2,
                data: create_test_rom_data(2048, 0xFF), // 2316 = 2KB
            })
            .expect("Failed to add file 2");

        let props = default_fw_props();
        let board = props.board();
        let flash_base = board.mcu_family().get_flash_base();
        let (metadata_buf, _rom_images_buf) = builder.build(props).expect("Build failed");

        // Parse metadata header
        let header = MetadataHeader::parse(&metadata_buf);
        header.validate_basic();

        // Validate we have 3 ROM sets
        assert_eq!(
            header.rom_set_count, 3,
            "Should have 3 ROM sets, got {}",
            header.rom_set_count
        );

        // Parse all three ROM sets
        let rom_set0_offset = (header.rom_sets_ptr - flash_base) as usize;
        let rom_set0 = RomSetStruct::parse(&metadata_buf, rom_set0_offset);

        let rom_set1_offset = rom_set0_offset + ROM_SET_METADATA_LEN;
        let rom_set1 = RomSetStruct::parse(&metadata_buf, rom_set1_offset);

        let rom_set2_offset = rom_set1_offset + ROM_SET_METADATA_LEN;
        let rom_set2 = RomSetStruct::parse(&metadata_buf, rom_set2_offset);

        // Validate data pointers are sequential
        let expected_data_ptr0 = flash_base + 65536;
        let expected_data_ptr1 = expected_data_ptr0 + 16384;
        let expected_data_ptr2 = expected_data_ptr1 + 16384;

        assert_eq!(rom_set0.data_ptr, expected_data_ptr0, "Set 0 data pointer");
        assert_eq!(rom_set1.data_ptr, expected_data_ptr1, "Set 1 data pointer");
        assert_eq!(rom_set2.data_ptr, expected_data_ptr2, "Set 2 data pointer");

        // Parse and validate ROM info for Set 0 (2364)
        let rom_array0_offset = (rom_set0.roms_ptr - flash_base) as usize;
        let rom_info0_ptr = u32::from_le_bytes([
            metadata_buf[rom_array0_offset],
            metadata_buf[rom_array0_offset + 1],
            metadata_buf[rom_array0_offset + 2],
            metadata_buf[rom_array0_offset + 3],
        ]);
        let rom_info0 = RomInfoStruct::parse(&metadata_buf, (rom_info0_ptr - flash_base) as usize);

        assert_eq!(rom_info0.rom_type, ROM_TYPE_2364);
        assert_eq!(rom_info0.cs1_state, CsLogic::ActiveLow.c_enum_val());

        // Parse and validate ROM info for Set 1 (2332)
        let rom_array1_offset = (rom_set1.roms_ptr - flash_base) as usize;
        let rom_info1_ptr = u32::from_le_bytes([
            metadata_buf[rom_array1_offset],
            metadata_buf[rom_array1_offset + 1],
            metadata_buf[rom_array1_offset + 2],
            metadata_buf[rom_array1_offset + 3],
        ]);
        let rom_info1 = RomInfoStruct::parse(&metadata_buf, (rom_info1_ptr - flash_base) as usize);

        assert_eq!(rom_info1.rom_type, ROM_TYPE_2332);
        assert_eq!(rom_info1.cs1_state, CsLogic::ActiveLow.c_enum_val());
        assert_eq!(rom_info1.cs2_state, CsLogic::ActiveHigh.c_enum_val());

        // Parse and validate ROM info for Set 2 (2316)
        let rom_array2_offset = (rom_set2.roms_ptr - flash_base) as usize;
        let rom_info2_ptr = u32::from_le_bytes([
            metadata_buf[rom_array2_offset],
            metadata_buf[rom_array2_offset + 1],
            metadata_buf[rom_array2_offset + 2],
            metadata_buf[rom_array2_offset + 3],
        ]);
        let rom_info2 = RomInfoStruct::parse(&metadata_buf, (rom_info2_ptr - flash_base) as usize);

        assert_eq!(rom_info2.rom_type, ROM_TYPE_2316);
        assert_eq!(rom_info2.cs1_state, CsLogic::ActiveLow.c_enum_val());
        assert_eq!(rom_info2.cs2_state, CsLogic::ActiveLow.c_enum_val());
        assert_eq!(rom_info2.cs3_state, CsLogic::ActiveLow.c_enum_val());

        println!("✓ Phase 2 Test 2: Three ROM sets validation passed");
        println!("  Set 0: 2364, CS1=Low");
        println!("  Set 1: 2332, CS1=Low, CS2=High");
        println!("  Set 2: 2316, CS1=Low, CS2=Low, CS3=Low");
        println!(
            "  Data pointers: 0x{:08X}, 0x{:08X}, 0x{:08X}",
            rom_set0.data_ptr, rom_set1.data_ptr, rom_set2.data_ptr
        );
    }

    // ========================================================================
    // PHASE 4: Boot Logging (Filenames)
    // ========================================================================

    // ========================================================================
    // TEST 4: Validate ROM Info with Filename
    // ========================================================================

    #[test]
    fn test_phase4_boot_logging_filename() {
        let json = r#"{
            "version": 1,
            "description": "Phase 4 boot logging test",
            "rom_sets": [{
                "type": "single",
                "roms": [{
                    "file": "test_filename.rom",
                    "type": "2364",
                    "cs1": "active_low"
                }]
            }]
        }"#;

        let mut builder = Builder::from_json(json).expect("Failed to parse JSON");
        let rom_data = create_test_rom_data(8192, 0xAA);
        builder
            .add_file(FileData {
                id: 0,
                data: rom_data,
            })
            .expect("Failed to add file");

        let props = fw_props_with_logging();
        let board = props.board();
        let flash_base = board.mcu_family().get_flash_base();
        let (metadata_buf, _rom_images_buf) = builder.build(props).expect("Build failed");

        // Parse metadata header and ROM set
        let header = MetadataHeader::parse(&metadata_buf);
        let rom_set_offset = (header.rom_sets_ptr - flash_base) as usize;
        let rom_set = RomSetStruct::parse(&metadata_buf, rom_set_offset);

        // Parse ROM pointer array to get pointer to first ROM info
        let rom_array_offset = (rom_set.roms_ptr - flash_base) as usize;
        let rom_info_ptr = u32::from_le_bytes([
            metadata_buf[rom_array_offset],
            metadata_buf[rom_array_offset + 1],
            metadata_buf[rom_array_offset + 2],
            metadata_buf[rom_array_offset + 3],
        ]);

        // Convert to buffer offset
        let rom_info_offset = (rom_info_ptr - flash_base) as usize;

        // Parse the ROM info structure WITH filename
        let rom_info = RomInfoStruct::parse_with_filename(&metadata_buf, rom_info_offset);

        // Validate basic ROM info fields (same as Phase 1)
        assert_eq!(rom_info.rom_type, ROM_TYPE_2364);
        assert_eq!(rom_info.cs1_state, CsLogic::ActiveLow.c_enum_val());
        assert_eq!(rom_info.cs2_state, CsLogic::Ignore.c_enum_val());
        assert_eq!(rom_info.cs3_state, CsLogic::Ignore.c_enum_val());

        // Validate filename pointer exists
        assert!(
            rom_info.filename_ptr.is_some(),
            "Filename pointer should be present with boot_logging enabled"
        );

        let filename_ptr = rom_info.filename_ptr.unwrap();

        // Validate filename pointer is within metadata buffer
        let filename_offset = (filename_ptr - flash_base) as usize;
        assert!(
            filename_offset < metadata_buf.len(),
            "Filename pointer {} (offset {}) outside metadata buffer (size {})",
            filename_ptr,
            filename_offset,
            metadata_buf.len()
        );

        // Parse the null-terminated filename string
        let filename = parse_null_terminated_string(&metadata_buf, filename_offset);

        // Validate filename matches what we specified in JSON
        assert_eq!(
            filename, "test_filename.rom",
            "Filename mismatch. Expected 'test_filename.rom', got '{}'",
            filename
        );

        println!("✓ Phase 4 Test 1: Boot logging with filename passed");
        println!("  - ROM type: {} (2364)", rom_info.rom_type);
        println!(
            "  - CS states: {}, {}, {}",
            rom_info.cs1_state, rom_info.cs2_state, rom_info.cs3_state
        );
        println!("  - Filename pointer: 0x{:08X}", filename_ptr);
        println!("  - Filename: '{}'", filename);
    }
}
