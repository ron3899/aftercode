use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Serialize, Deserialize)]
struct Creds {
    token: String,
}

pub fn creds_path() -> anyhow::Result<PathBuf> {
    let base = dirs::config_dir().ok_or_else(|| anyhow::anyhow!("no config dir"))?;
    Ok(base.join("aftercode").join("credentials.json"))
}

pub fn save_token(token: &str) -> anyhow::Result<()> {
    let p = creds_path()?;
    if let Some(parent) = p.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(
        &p,
        serde_json::to_string(&Creds {
            token: token.into(),
        })?,
    )?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o600))?;
    }
    Ok(())
}

pub fn load_token() -> anyhow::Result<String> {
    let p = creds_path()?;
    let txt = std::fs::read_to_string(&p)
        .map_err(|_| anyhow::anyhow!("not logged in — run `aftercode login <token>`"))?;
    let c: Creds = serde_json::from_str(&txt)?;
    Ok(c.token)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    #[serial_test::serial(fs)]
    fn save_then_load() {
        let dir = tempfile::tempdir().unwrap();
        std::env::set_var("XDG_CONFIG_HOME", dir.path());
        save_token("ak_abc").unwrap();
        assert_eq!(load_token().unwrap(), "ak_abc");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(creds_path().unwrap())
                .unwrap()
                .permissions()
                .mode();
            assert_eq!(mode & 0o777, 0o600);
        }
    }
}
