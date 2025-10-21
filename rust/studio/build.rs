// Copyright (C) 2025 Piers Finlayson <piers@piers.rocks>
//
// MIT License

use git2::Repository;

fn main() {
    // Minimum macOS deployment target
    println!("cargo:rustc-env=MACOSX_DEPLOYMENT_TARGET=10.13");

    // Re-run build script if build.rs changes
    println!("cargo:rerun-if-changed=build.rs");

    // Write built-time information
    built::write_built_file().expect("Failed to acquire build-time information");
    
    // Compile Windows resources
    if std::env::var("CARGO_CFG_TARGET_OS").unwrap() == "windows" {
        compile_windows_resources();
    }
}

fn compile_windows_resources() {
    // Create a version string, including git commit if available
    let commit_id = if let Ok(repo) = Repository::open("../..") {
        let head = repo.head().unwrap();
        let commit = head.peel_to_commit().unwrap();
        let commit_id = commit.id().to_string()[..7].to_string();

        let statuses = repo.statuses(None).unwrap();
        let is_dirty = !statuses.is_empty();

        format!(
            " ({}{})",
            if is_dirty { "!" } else { "" },
            &commit_id,
        )
    } else {
        println!("cargo:warning=Could not open git repository to get commit ID");
        "".to_string()
    };
    let version = format!(
        "v{}{}",
        env!("CARGO_PKG_VERSION"),
        commit_id,
    );

    // Get absolute path to icon
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let icon_path = format!("{}/assets/icon.ico", manifest_dir);

    // Create RC file content
    let rc_content = format!(r#"1 ICON "{}"

1 VERSIONINFO
FILEVERSION 0,1,0,0
PRODUCTVERSION 0,1,0,0
{{
  BLOCK "StringFileInfo"
  {{
    BLOCK "040904E4"
    {{
      VALUE "ProductName", "One ROM Studio"
      VALUE "FileDescription", "One ROM Studio - Manage One ROM"
      VALUE "FileVersion", "{}"
      VALUE "ProductVersion", "{}"
      VALUE "OriginalFilename", "onerom-studio.exe"
      VALUE "LegalCopyright", "2025 Piers Finlayson"
      VALUE "CompanyName", "piers.rocks"
    }}
  }}
  BLOCK "VarFileInfo"
  {{
    VALUE "Translation", 0x409, 1252
  }}
}}"#, icon_path, version, version);

    let out_dir = std::env::var("OUT_DIR").unwrap();
    let rc_path = format!("{}/resources.rc", out_dir);
    std::fs::write(&rc_path, rc_content).unwrap();
    
    let result = embed_resource::compile(&rc_path, embed_resource::NONE);
    if result != embed_resource::CompilationResult::Ok {
        panic!("Failed to compile Windows resources {result}");
    }
    println!("cargo:warning=Windows resources compiled with version: {}", version);
}