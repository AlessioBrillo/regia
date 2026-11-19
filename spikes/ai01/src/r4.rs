//! R4: Mistral cloud (EU). Synthetic input only; the router refuses canary, `restricted` and `secret` before this
//! adapter is ever called. The key comes from the environment and is never logged.

use serde_json::{Value, json};

use crate::{
    contract::{Adapter, Proposal, TaskRequest},
    llm,
};

/// A Mistral chat model.
pub struct Mistral {
    url: String,
    model: String,
    version: String,
    key: String,
    agent: ureq::Agent,
}

/// Text and token counts (in, out) of a chat-completions reply.
///
/// # Errors
/// The reply has no message text.
pub fn content(v: &Value) -> Result<(String, u64, u64), String> {
    let text = v["choices"][0]["message"]["content"]
        .as_str()
        .ok_or("reply without message content")?;
    Ok((
        text.to_owned(),
        v["usage"]["prompt_tokens"].as_u64().unwrap_or(0),
        v["usage"]["completion_tokens"].as_u64().unwrap_or(0),
    ))
}

impl Mistral {
    /// Reads `MISTRAL_API_KEY` (and optionally `MISTRAL_URL`).
    ///
    /// # Errors
    /// No key in the environment.
    pub fn new(model: &str) -> Result<Self, String> {
        let key = std::env::var("MISTRAL_API_KEY").map_err(|_| "MISTRAL_API_KEY is not set")?;
        let url = std::env::var("MISTRAL_URL").unwrap_or_else(|_| "https://api.mistral.ai".into());
        Ok(Self {
            url: url.trim_end_matches('/').to_owned(),
            model: model.to_owned(),
            version: format!("{model}:{}", llm::PROMPT_VERSION),
            key,
            agent: llm::agent(),
        })
    }
}

impl Adapter for Mistral {
    fn id(&self) -> &'static str {
        "r4"
    }
    fn version(&self) -> &str {
        &self.version
    }
    fn local(&self) -> bool {
        false
    }
    fn extract(&self, req: &TaskRequest) -> Result<Proposal, String> {
        let body = json!({
            "model": self.model,
            "temperature": 0,
            "response_format": {"type": "json_object"},
            "messages": llm::messages(req),
        });
        let reply: Value = self
            .agent
            .post(format!("{}/v1/chat/completions", self.url))
            .header("Authorization", format!("Bearer {}", self.key))
            .send_json(&body)
            .map_err(|e| e.to_string())?
            .body_mut()
            .read_json()
            .map_err(|e| e.to_string())?;
        let (text, tin, tout) = content(&reply)?;
        llm::add_tokens(tin, tout);
        llm::parse(&text)
    }
}
