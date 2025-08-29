// Copyright (c) 2025 Piers Finlayson <piers@piers.rocks>
//
// MIT licence

use std::env;
use std::fs;
use std::path::Path;

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

    // Generate the ROM database
    generate_rom_db();
}

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
                                eprintln!("cargo:warning=Invalid UTF-8 in path: {}", path.display());
                            }
                        }
                    }
                    Err(e) => eprintln!("cargo:warning=Failed to read directory entry: {}", e),
                }
            }
        }
        Err(e) => eprintln!("cargo:warning=Failed to read roms directory: {}", e),
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

fn process_rom_csv(filename: &str) -> Vec<String> {
    let csv_data = fs::read_to_string(filename)
        .expect(&format!("Failed to read {}", filename));
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(true)
        .flexible(true)
        .from_reader(csv_data.as_bytes());
    
    let mut entries = Vec::new();
    
    for (line_num, result) in reader.records().enumerate() {
        let record = result.expect(&format!("Failed to parse CSV record at line {}", line_num + 2));
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
            "" => panic!("Missing CS value for {} ({})", name, part),
            _ => panic!("Invalid CS value '{}' for {} ({})", val, name, part),
        };

        let rom_type_code = match rom_type {
            "2364" => format!("RomType::Type2364{{ cs: CsActive::{} }}", cs_active(cs1)),
            "2332" => format!("RomType::Type2332{{ cs1: CsActive::{}, cs2: CsActive::{} }}", cs_active(cs1), cs_active(cs2)),
            "2316" => format!("RomType::Type2316{{ cs1: CsActive::{}, cs2: CsActive::{}, cs3: CsActive::{} }}", cs_active(cs1), cs_active(cs2), cs_active(cs3)),
            _ => panic!("Unknown ROM type '{}' for {} ({})", rom_type, name, part),
        };
        
        entries.push(format!(
            r#"    Entry::new(
        "{}",
        "{}",
        {},
        hex!("{}"),
        {},
    ),"#,
            name, part, checksum, sha1, rom_type_code
        ));
    }
    
    entries
}

