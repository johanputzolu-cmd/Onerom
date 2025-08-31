// Copyright (c) 2025 Piers Finlayson <piers@piers.rocks>
//
// MIT licence

use std::env;
use std::fs;
use std::path::Path;
use std::os::unix::fs::PermissionsExt;

fn main() {
    // Re-run this build script if anything in git changes.
    println!("cargo:rerun-if-changed=.git/HEAD");
    println!("cargo:rerun-if-changed=.git/refs/");

    // Set up STM32 linking
    println!("cargo:rustc-link-arg=-v");
    println!("cargo:rustc-link-arg-bins=--nmagic");
    println!("cargo:rustc-link-arg-bins=-Tlink.x");

    // Re-run this build script of DEFMT_LOG changes.
    println!("cargo:rerun-if-env-changed=DEFMT_LOG");

    // Required for defmt
    println!("cargo:rustc-link-arg-bins=-Tdefmt.x");

    // Re-run if the ROMs change
    println!("cargo:rerun-if-changed=roms");

    // Set the cargo runner
    set_cargo_runner();

    // Generate memory.x
    generate_memory_x();

    // Generate the ROM database
    generate_rom_db();

    // Generate built information
    built::write_built_file().expect("Failed to acquire build-time information");
}

fn set_cargo_runner() {
    const RUN_CMD_PREFIX: &str = "probe-rs run --no-location --chip ";

    let chip_id = if cfg!(feature = "f401re") {
        "STM32F401RETx"
    } else if cfg!(feature = "f405rg") {
        "STM32F405RGTx"
    } else if cfg!(feature = "f411re") {
        "STM32F411RETx"
    } else if cfg!(feature = "f446re") {
        "STM32F446RETx"
    } else {
        panic!("Unknown hardware variant - perhaps you need to update `set_cargo_runner()` in `build.rs`?");
    };

    // Create the script to run the binary using probe-rs
    let runner_cmd = format!("{}{}", RUN_CMD_PREFIX, chip_id);
    let script = format!(r#"#!/bin/bash
echo "-----"
echo Running {runner_cmd} "$@"
echo "-----"
{runner_cmd} "$@"
"#
    );

    let out_dir = env::var("OUT_DIR").unwrap();
    let runner_path = format!("{}/runner.sh", out_dir);

    fs::write(&runner_path, script).unwrap();
    fs::set_permissions(&runner_path, fs::Permissions::from_mode(0o755)).unwrap();
}

// Consts for `generate_memory_x()`
const STM32_FLASH_START: usize = 0x08000000;
const STM32_RAM_START: usize = 0x20000000;
const AIRFROG_FLASH_LOOKUP_OFFSET: usize = 0x200;
const AIRFROG_RAM_LOOKUP_OFFSET: usize = 0x0;
const FLASH_INFO_START: usize = STM32_FLASH_START + AIRFROG_FLASH_LOOKUP_OFFSET;
const RAM_INFO_START: usize = STM32_RAM_START + AIRFROG_RAM_LOOKUP_OFFSET;
const FLASH_INFO_SIZE: usize = 256;
const RAM_INFO_SIZE: usize = 256;
const FLASH_INFO_SECTION: &str = ".lab_flash_info";
const RAM_INFO_SECTION: &str = ".lab_ram_info";
const POST_FLASH_INFO: usize = FLASH_INFO_START + FLASH_INFO_SIZE;

// Creates a custom memory.x file for this firmware.  We do this so we can
// place LAB_FLASH_INFO at a 0x200 offset from the start of flash, and
// LAB_RAM_INFO at the beginning of RAM.  This allows Airfrog to find it and
// decode the firmware and runtime information.
//
// This works by leveraging cortex_m_rt's link.x flexibility, to jiggle stuff
// around.  We leave the .vector_table in place (it has to be first in flash
// for the STM32), but push .data out from the start of RAM so we can have it.
fn generate_memory_x() {
    let out_dir = env::var("OUT_DIR").unwrap();
    let memory_path = Path::new(&out_dir).join("memory.x");

    let memory_x = format!(
        r#"
/* Standard STM32F405RG memory layout */
MEMORY
{{
    FLASH : ORIGIN = {STM32_FLASH_START:#010X}, LENGTH = 1024K
    RAM   : ORIGIN = {STM32_RAM_START:#010X}, LENGTH = 128K - {RAM_INFO_SIZE:#020X}
}}

/* Section to store firmware information to flash */ 
SECTIONS
{{
    {FLASH_INFO_SECTION} {FLASH_INFO_START:#010X} : AT({FLASH_INFO_START:#010X}) {{
        *({FLASH_INFO_SECTION}*)
    }} > FLASH
}}
INSERT AFTER .vector_table

/* Force .text to start after {FLASH_INFO_SECTION} */
PROVIDE(_stext = {POST_FLASH_INFO:#010X});

/* Section to store runtime information in RAM */
SECTIONS
{{
    {RAM_INFO_SECTION} {RAM_INFO_START:#010X} : AT({RAM_INFO_START:#010X}) {{
        *({RAM_INFO_SECTION}*)
    }} > RAM
}}
INSERT AFTER .rodata;

_SEGGER_RTT_ADDRESS = ABSOLUTE(_SEGGER_RTT);
"#
    );

    fs::write(memory_path, memory_x).unwrap();

    println!("cargo:rustc-link-search={out_dir}");
}

// Create the ROM database, by parsing all files in `roms/`.  This is then
// included by `src/database.rs`.
fn generate_rom_db() {
    let out_dir = env::var("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("roms.rs");

    let mut entries = Vec::new();

    // Process all CSV files in roms directory
    let roms_dir = Path::new("roms");
    match fs::read_dir(roms_dir) {
        Ok(dir_entries) => {
            for entry in dir_entries {
                match entry {
                    Ok(entry) => {
                        let path = entry.path();
                        if path.extension().and_then(|s| s.to_str()) == Some("csv") {
                            if let Some(path_str) = path.to_str() {
                                entries.extend(process_rom_csv(path_str));
                            } else {
                                eprintln!(
                                    "cargo:warning=Invalid UTF-8 in path: {}",
                                    path.display()
                                );
                            }
                        }
                    }
                    Err(e) => eprintln!("cargo:warning=Failed to read directory entry: {e}"),
                }
            }
        }
        Err(e) => eprintln!("cargo:warning=Failed to read roms directory: {e}"),
    }

    // Generate the database
    let generated = format!(
        r#"pub const ROMS: &[Entry] = &[
    {}
];"#,
        entries.join("\n")
    );

    fs::write(&dest_path, generated).expect("Failed to write ROMS database");
}

// Create ROM database entries for a single CSV file
fn process_rom_csv(filename: &str) -> Vec<String> {
    let csv_data =
        fs::read_to_string(filename).unwrap_or_else(|_| panic!("Failed to read {filename}"));
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(true)
        .flexible(true)
        .from_reader(csv_data.as_bytes());

    let mut entries = Vec::new();

    for (line_num, result) in reader.records().enumerate() {
        let record = result
            .unwrap_or_else(|_| panic!("Failed to parse CSV record at line {}", line_num + 2));
        let name = &record[0];
        let part = &record[1];
        let checksum = &record[2];
        let sha1 = &record[3];
        let rom_type = &record[4];
        let cs1 = &record[5];
        let cs2 = record.get(6).unwrap_or("");
        let cs3 = record.get(7).unwrap_or("");

        let cs_active = |val: &str| match val {
            "0" => "Low",
            "1" => "High",
            "" => panic!("Missing CS value for {name} ({part})"),
            _ => panic!("Invalid CS value '{val}' for {name} ({part})"),
        };

        let rom_type_code = match rom_type {
            "2364" => format!("RomType::Type2364{{ cs: CsActive::{} }}", cs_active(cs1)),
            "2332" => format!(
                "RomType::Type2332{{ cs1: CsActive::{}, cs2: CsActive::{} }}",
                cs_active(cs1),
                cs_active(cs2)
            ),
            "2316" => format!(
                "RomType::Type2316{{ cs1: CsActive::{}, cs2: CsActive::{}, cs3: CsActive::{} }}",
                cs_active(cs1),
                cs_active(cs2),
                cs_active(cs3)
            ),
            _ => panic!("Unknown ROM type '{rom_type}' for {name} ({part})"),
        };

        entries.push(format!(
            r#"    Entry::new(
        "{name}",
        "{part}",
        {checksum},
        hex!("{sha1}"),
        {rom_type_code},
    ),"#
        ));
    }

    entries
}
