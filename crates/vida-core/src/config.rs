use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub language: String,
    pub theme: String,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            language: "en".to_string(),
            theme: "dark".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = AppConfig::default();
        assert_eq!(config.language, "en");
        assert_eq!(config.theme, "dark");
    }

    #[test]
    fn test_config_serialization() {
        let config = AppConfig {
            language: "fr".to_string(),
            theme: "light".to_string(),
        };
        let json = serde_json::to_string(&config).unwrap();
        let deserialized: AppConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.language, "fr");
    }
}
