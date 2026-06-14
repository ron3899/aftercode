use aftercode_core::session::SessionContext;

pub struct Client {
    base: String,
    token: String,
    http: reqwest::Client,
}

impl Client {
    pub fn new(base: String, token: String) -> Self {
        Client {
            base,
            token,
            http: reqwest::Client::new(),
        }
    }

    fn url(&self, path: &str) -> String {
        format!("{}{}", self.base.trim_end_matches('/'), path)
    }

    pub async fn register_project(&self, name: &str, language: &str) -> anyhow::Result<String> {
        let v: serde_json::Value = self
            .http
            .post(self.url("/cli/register-project"))
            .bearer_auth(&self.token)
            .json(&serde_json::json!({ "name": name, "default_language": language }))
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        Ok(v["project_id"].as_str().unwrap_or_default().to_string())
    }

    pub async fn generate_episode(
        &self,
        ctx: &SessionContext,
        language: &str,
    ) -> anyhow::Result<String> {
        let v: serde_json::Value = self
            .http
            .post(self.url("/cli/generate-episode"))
            .bearer_auth(&self.token)
            .json(&serde_json::json!({
                "project_id": ctx.project_id, "language": language, "session_context": ctx }))
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        Ok(v["episode_id"].as_str().unwrap_or_default().to_string())
    }

    pub async fn episode_status(&self, id: &str) -> anyhow::Result<serde_json::Value> {
        Ok(self
            .http
            .get(self.url(&format!("/cli/episode-status/{id}")))
            .bearer_auth(&self.token)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?)
    }
}
