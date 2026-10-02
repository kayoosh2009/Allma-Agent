use serde::{Deserialize, Serialize};
use std::{sync::mpsc::Sender, time::Duration};

#[derive(Serialize)]
pub struct ChatMsg {
    pub role: String,
    pub content: String,
}

#[derive(Deserialize)]
struct Resp {
    message: RespMsg,
    #[serde(default)]
    prompt_eval_count: u64,
    #[serde(default)]
    eval_count: u64,
}

#[derive(Deserialize)]
struct RespMsg {
    content: String,
}

pub enum AiEvent {
    Reply { text: String, tokens: u64 },
    Error(String),
}

pub fn ask(api_key: String, model: String, history: Vec<ChatMsg>, tx: Sender<AiEvent>) {
    std::thread::spawn(move || {
        let ev = match request(&api_key, &model, &history) {
            Ok((text, tokens)) => AiEvent::Reply { text, tokens },
            Err(e) => AiEvent::Error(e),
        };
        let _ = tx.send(ev);
    });
}

fn request(key: &str, model: &str, history: &[ChatMsg]) -> Result<(String, u64), String> {
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(300))
        .build()
        .map_err(|e| e.to_string())?;

    let resp = client
        .post("https://ollama.com/api/chat")
        .bearer_auth(key)
        .json(&serde_json::json!({
            "model": model,
            "messages": history,
            "stream": false
        }))
        .send()
        .map_err(|e| e.to_string())?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().unwrap_or_default();
        return Err(format!("{status}: {body}"));
    }

    let r: Resp = resp.json().map_err(|e| e.to_string())?;
    Ok((r.message.content, r.prompt_eval_count + r.eval_count))
}