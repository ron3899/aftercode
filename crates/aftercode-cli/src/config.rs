use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Privacy {
    pub ignore_paths: Vec<String>,
    pub send_raw_code: bool,
    pub send_diffs: bool,
}

impl Default for Privacy {
    fn default() -> Self {
        Privacy {
            ignore_paths: vec![
                ".env".into(),
                "node_modules".into(),
                "dist".into(),
                "build".into(),
            ],
            send_raw_code: false,
            send_diffs: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub project_id: String,
    pub project_name: String,
    pub language: String,
    pub episode_length_minutes: u8,
    pub api_base_url: String,
    #[serde(default)]
    pub privacy: Privacy,
}

pub fn config_path() -> PathBuf {
    Path::new(".aftercode").join("config.json")
}

impl Config {
    pub fn load() -> anyhow::Result<Self> {
        let p = config_path();
        let txt = std::fs::read_to_string(&p)
            .map_err(|_| anyhow::anyhow!("no .aftercode/config.json — run `aftercode init`"))?;
        Ok(serde_json::from_str(&txt)?)
    }
    pub fn save(&self) -> anyhow::Result<()> {
        std::fs::create_dir_all(".aftercode")?;
        std::fs::write(config_path(), serde_json::to_string_pretty(self)?)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    #[serial_test::serial(fs)]
    fn roundtrip_in_tempdir() {
        let dir = tempfile::tempdir().unwrap();
        let prev = std::env::current_dir().unwrap();
        std::env::set_current_dir(dir.path()).unwrap();
        let c = Config {
            project_id: "proj_1".into(),
            project_name: "p".into(),
            language: "en".into(),
            episode_length_minutes: 10,
            api_base_url: "http://localhost:8080".into(),
            privacy: Privacy::default(),
        };
        c.save().unwrap();
        let back = Config::load().unwrap();
        std::env::set_current_dir(prev).unwrap();
        assert_eq!(back.project_id, "proj_1");
        assert_eq!(back.privacy.ignore_paths.len(), 4);
    }
}
