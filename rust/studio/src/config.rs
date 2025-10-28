// Copyright (C) 2025 Piers Finlayson <piers@piers.rocks>
//
// MIT License

//! Handles ROM metadata and image JSON format config files

// Configuration site base URL and manifest
const CONFIG_SITE_BASE: &str = "images.onerom.org";
const CONFIG_MANIFEST: &str = "configs.json";

/// Structure representing all available ROM configuration files
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Configs {
    /// Version of the manifest
    pub version: usize,

    /// List of configuration file paths
    pub configs: Vec<String>,

    /// List of configuration names (derived from the manifest paths)
    #[serde(skip)]
    pub names: Option<Vec<String>>,
}

impl Configs {
    /// Create a new Configs instance from JSON manifest file
    pub fn from_json(json: String) -> Result<Self, String> {
        // Parse the JSON
        let mut configs: Configs = serde_json::from_str(&json)
            .map_err(|e| format!("Failed to parse Configs JSON:\n  - {e}"))?;

        // Create names (required by pick list) and sort alphabetically
        let mut names = configs
            .configs
            .iter()
            .filter_map(|c| {
                let file_name = c.split('/').last()?.split('.').next()?;
                Some(file_name.to_string())
            })
            .collect::<Vec<_>>();
        names.sort_by_key(|name| (name.to_lowercase() != "blank", name.to_lowercase()));
        configs.names = Some(names);

        Ok(configs)
    }
}

impl std::fmt::Display for Configs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Configs({})", self.configs.len(),)
    }
}

impl Configs {
    /// Create Configs from network manifest
    pub async fn from_network_async() -> Result<Self, String> {
        // Get the manifest from the network
        let url = Self::manifest_url();
        let response = reqwest::get(&url)
            .await
            .map_err(|e| format!("Network error fetching Configs manifest:\n  - {e}"))?;
        let text = response
            .text()
            .await
            .map_err(|e| format!("Network error reading Configs manifest:\n  - {e}"))?;

        // Construct from JSON
        Self::from_json(text)
    }

    /// Return names of the configs
    pub fn names(&self) -> &Vec<String> {
        // The config string is path/to/name.json.
        // We want to extract the name without the path and extension.
        &self.names.as_ref().unwrap()
    }

    /// Return names of the configs as a single string with commas
    pub fn names_str(&self) -> String {
        self.names().join(", ")
    }

    /// Return config URL for a given name
    pub fn config_url(&self, name: &str) -> Option<String> {
        let path = self.path(name)?;
        Some(format!("https://{}/{}", CONFIG_SITE_BASE, path))
    }

    // Return path for config of a given name
    fn path(&self, name: &str) -> Option<String> {
        for c in &self.configs {
            let file_name = c.split('/').last()?.split('.').next()?;
            if file_name == name {
                return Some(c.clone());
            }
        }
        None
    }

    /// Return configs manifest URL
    fn manifest_url() -> String {
        format!("https://{}/{}", CONFIG_SITE_BASE, CONFIG_MANIFEST)
    }
}

/// Fetch config file from URL
pub async fn get_config_from_url(url: &String) -> Result<Vec<u8>, String> {
    let response = reqwest::get(url)
        .await
        .map_err(|e| format!("Network error fetching Config:\n  - {e}"))?;

    let bytes = response
        .bytes()
        .await
        .map_err(|e| format!("Network error reading Config:\n  - {e}"))?;

    Ok(bytes.to_vec())
}
