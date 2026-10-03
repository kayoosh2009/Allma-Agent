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

/// Системный промпт: твой prompt.txt + дневник + служебная инструкция про <diary>.
/// Инструкция зашита в код, чтобы правка prompt.txt не сломала механизм дневника.
pub fn system_prompt(prompt: &str, diary: &str) -> String {
    format!(
        "{prompt}\n\n\
         # Дневник\n\
         Ниже твои записи о собеседнике. Опирайся на них в разговоре.\n\n\
         {diary}\n\n\
         # Как вести дневник\n\
         Узнав о собеседнике что-то новое и важное (имя, проекты, стек, предпочтения, привычки), \
         добавь в ответ тег <diary>короткая заметка</diary>. \
         Одна заметка — один факт. Не повторяй то, что уже есть в дневнике. \
         Не записывай мелочи и пароли/ключи. Тег пользователь не увидит."
    )
}

/// Вырезает из ответа теги <diary>…</diary>, возвращает (чистый текст, заметки).
pub fn extract_diary(text: &str) -> (String, Vec<String>) {
    let mut clean = String::new();
    let mut notes = vec![];
    let mut rest = text;
    while let Some(start) = rest.find("<diary>") {
        let after = &rest[start + 7..];
        let Some(end) = after.find("</diary>") else { break };
        clean.push_str(&rest[..start]);
        let note = after[..end].trim();
        if !note.is_empty() {
            notes.push(note.to_string());
        }
        rest = &after[end + 8..];
    }
    clean.push_str(rest);
    (clean.trim().to_string(), notes)
}