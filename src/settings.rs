//! Settings persistence and embedded model configuration

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Application settings persisted to JSON file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub monitor_enabled: bool,
    pub current_voice: String,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            monitor_enabled: true,
            current_voice: "alba".to_string(),
        }
    }
}

impl Settings {
    /// Get the path to the settings file (next to executable)
    pub fn config_path() -> anyhow::Result<PathBuf> {
        let exe = std::env::current_exe()?;
        let dir = exe.parent().ok_or_else(|| anyhow::anyhow!("No parent directory"))?;
        Ok(dir.join("pocket-tray.json"))
    }

    /// Load settings from file or return default
    pub fn load_or_default() -> Self {
        Self::config_path()
            .ok()
            .and_then(|p| std::fs::read_to_string(p).ok())
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    /// Save settings to file
    pub fn save(&self) -> anyhow::Result<()> {
        let path = Self::config_path()?;
        let contents = serde_json::to_string_pretty(self)?;
        std::fs::write(path, contents)?;
        Ok(())
    }
}

/// Get the models directory path (next to executable)
pub fn models_dir() -> anyhow::Result<PathBuf> {
    let exe = std::env::current_exe()?;
    let dir = exe.parent().ok_or_else(|| anyhow::anyhow!("No parent directory"))?;
    Ok(dir.join("models"))
}

/// List of available voices
pub const VOICES: &[&str] = &[
    "alba",
    "azelma",
    "cosette",
    "eponine",
    "fantine",
    "javert",
    "jean",
    "marius",
];

/// Create embedded model configuration matching b6369a24.yaml
/// This avoids needing to ship/parse a YAML file at runtime
pub fn embedded_config() -> pocket_tts::config::Config {
    pocket_tts::config::Config {
        weights_path: None,
        weights_path_without_voice_cloning: None,
        flow_lm: pocket_tts::config::FlowLMConfig {
            dtype: "float32".to_string(),
            weights_path: None,
            flow: pocket_tts::config::FlowConfig {
                dim: 512,
                depth: 6,
            },
            transformer: pocket_tts::config::FlowLMTransformerConfig {
                d_model: 1024,
                hidden_scale: 4,
                max_period: 10000,
                num_heads: 16,
                num_layers: 6,
            },
            lookup_table: pocket_tts::config::LookupTableConfig {
                dim: 1024,
                n_bins: 4000,
                tokenizer: "sentencepiece".to_string(),
                tokenizer_path: String::new(), // Not used in offline mode
            },
        },
        mimi: pocket_tts::config::MimiConfig {
            dtype: "float32".to_string(),
            sample_rate: 24000,
            channels: 1,
            frame_rate: 12.5,
            weights_path: None,
            seanet: pocket_tts::config::SEANetConfig {
                dimension: 512,
                channels: 1,
                n_filters: 64,
                n_residual_layers: 1,
                ratios: vec![6, 5, 4],
                kernel_size: 7,
                residual_kernel_size: 3,
                last_kernel_size: 3,
                dilation_base: 2,
                pad_mode: "constant".to_string(),
                compress: 2,
            },
            transformer: pocket_tts::config::MimiTransformerConfig {
                d_model: 512,
                num_heads: 8,
                num_layers: 2,
                layer_scale: 0.01,
                context: 250,
                max_period: 10000.0,
                dim_feedforward: 2048,
                input_dimension: 512,
                output_dimensions: vec![512],
            },
            quantizer: pocket_tts::config::QuantizerConfig {
                dimension: 32,
                output_dimension: 512,
            },
        },
    }
}
