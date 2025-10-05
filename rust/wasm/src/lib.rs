// Copyright (C) 2025 Piers Finlayson <piers@piers.rocks>
//
// MIT License

/// Web Assembly bindings for One ROM supporting tooling.

use wasm_bindgen::prelude::*;
use sdrr_fw_parser::{Parser, readers::MemoryReader};

#[wasm_bindgen(start)]
pub fn init() {
    console_error_panic_hook::set_once();
    console_log::init_with_level(log::Level::Debug).unwrap();
}

#[wasm_bindgen]
pub async fn parse_firmware(data: Vec<u8>) -> Result<JsValue, JsValue> {
    let mut reader = MemoryReader::new(data, 0x08000000);
    let mut parser = Parser::new(&mut reader);
    
    let info = parser.parse_flash()
        .await
        .map_err(|e| JsValue::from_str(&e))?;
    
    // Serialize to JSON
    serde_wasm_bindgen::to_value(&info)
        .map_err(|e| JsValue::from_str(&e.to_string()))
}
