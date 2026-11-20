//! R3: a local open-weight model served by Ollama. Never leaves the device.

use serde_json::{Value, json};

use crate::{
    contract::{Adapter, Proposal, TaskRequest},
    llm,
};

// Explicit, so Ollama's smaller default context cannot silently drop the start of the prompt. p1 is about 700
// tokens and the longest golden input is short; both values are recorded in `version`.
const NUM_CTX: u32 = 4096;
const NUM_PREDICT: u32 = 1024;

/// An Ollama model. `version` carries the model tag, the exact digest and the prompt version.
pub struct Ollama {
    url: String,
    model: String,
    version: String,
    agent: ureq::Agent,
}

fn get(agent: &ureq::Agent, url: &str) -> Result<Value, String> {
    agent
        .get(url)
        .call()
        .map_err(|e| e.to_string())?
        .body_mut()
        .read_json()
        .map_err(|e| e.to_string())
}

/// Text and token counts (in, out) of an `/api/chat` reply.
///
/// # Errors
/// The reply has no message text.
pub fn content(v: &Value) -> Result<(String, u64, u64), String> {
    let text = v["message"]["content"]
        .as_str()
        .ok_or("reply without message content")?;
    Ok((
        text.to_owned(),
        v["prompt_eval_count"].as_u64().unwrap_or(0),
        v["eval_count"].as_u64().unwrap_or(0),
    ))
}

impl Ollama {
    /// Connects and reads the model digest.
    ///
    /// # Errors
    /// Ollama is unreachable or the model is not pulled: a result for the support matrix, not a reason to swap.
    pub fn new(url: &str, model: &str) -> Result<Self, String> {
        let agent = llm::agent();
        let url = url.trim_end_matches('/');
        let tags = get(&agent, &format!("{url}/api/tags"))?;
        let latest = format!("{model}:latest");
        let found = tags["models"]
            .as_array()
            .into_iter()
            .flatten()
            .find(|m| m["name"] == model || m["name"] == latest.as_str())
            .ok_or_else(|| format!("model {model} is not pulled"))?;
        let name = found["name"].as_str().unwrap_or(model);
        let digest = found["digest"].as_str().ok_or("model without digest")?;
        Ok(Self {
            url: url.to_owned(),
            model: name.to_owned(),
            version: format!(
                "{name}@{digest}:{}:ctx{NUM_CTX}:out{NUM_PREDICT}",
                llm::PROMPT_VERSION
            ),
            agent,
        })
    }

    /// Bytes this model occupies right now (`/api/ps`), for the RAM metric.
    #[must_use]
    pub fn loaded_bytes(&self) -> Option<u64> {
        get(&self.agent, &format!("{}/api/ps", self.url)).ok()?["models"]
            .as_array()?
            .iter()
            .find(|m| m["name"] == self.model.as_str())?["size"]
            .as_u64()
    }
}

impl Adapter for Ollama {
    fn id(&self) -> &'static str {
        "r3"
    }
    fn version(&self) -> &str {
        &self.version
    }
    fn local(&self) -> bool {
        true
    }
    fn extract(&self, req: &TaskRequest) -> Result<Proposal, String> {
        let body = json!({
            "model": self.model,
            "stream": false,
            "format": "json",
            "options": {"temperature": 0, "num_ctx": NUM_CTX, "num_predict": NUM_PREDICT},
            "messages": llm::messages(req),
        });
        let reply: Value = self
            .agent
            .post(format!("{}/api/chat", self.url))
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
