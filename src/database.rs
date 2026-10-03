use crate::ui::Stats;
use rusqlite::{params, Connection, OptionalExtension, Result};
use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

const DAY: i64 = 86_400;
const DEFAULT_PROMPT: &str = "Ты — Allma, дружелюбный ИИ-помощник и друг разработчика.
Общайся тепло и по-человечески, на «ты», без лишней официальности.
Отвечай на языке собеседника, по делу и без воды.
Код оформляй блоками, объясняй коротко и понятно.
Если чего-то не знаешь — честно скажи об этом.
";

pub struct Db {
    conn: Connection,
    dir: PathBuf,
}

fn now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

impl Db {
    pub fn open() -> std::result::Result<Self, String> {
        let dir = dirs::data_dir()
            .ok_or("не найдена папка данных")?
            .join("allma-agent");
        fs::create_dir_all(&dir).map_err(|e| e.to_string())?;

        let db_path = dir.join("allma.db");
        let conn = Connection::open(&db_path).map_err(|e| e.to_string())?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS messages (
                id      INTEGER PRIMARY KEY,
                role    TEXT    NOT NULL,
                content TEXT    NOT NULL,
                tokens  INTEGER NOT NULL DEFAULT 0,
                ts      INTEGER NOT NULL
            );
            CREATE TABLE IF NOT EXISTS settings (
                key   TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );",
        )
        .map_err(|e| e.to_string())?;

        // в базе лежит API-ключ, поэтому читать её должен только владелец
        let _ = fs::set_permissions(&db_path, fs::Permissions::from_mode(0o600));

        let db = Self { conn, dir };
        if !db.prompt_path().exists() {
            fs::write(db.prompt_path(), DEFAULT_PROMPT).map_err(|e| e.to_string())?;
        }
        if !db.diary_path().exists() {
            fs::write(db.diary_path(), "# Дневник\n\n").map_err(|e| e.to_string())?;
        }
        Ok(db)
    }

    // ---------- сообщения ----------

    pub fn add_message(&self, from_user: bool, text: &str, tokens: u64) -> Result<()> {
        self.conn.execute(
            "INSERT INTO messages (role, content, tokens, ts) VALUES (?1, ?2, ?3, ?4)",
            params![
                if from_user { "user" } else { "assistant" },
                text,
                tokens as i64,
                now()
            ],
        )?;
        Ok(())
    }

    pub fn load_messages(&self) -> Vec<(bool, String)> {
        let from: i64 = self
            .get_setting("clear_id")
            .and_then(|v| v.parse().ok())
            .unwrap_or(0);
        let Ok(mut st) = self
            .conn
            .prepare("SELECT role, content FROM messages WHERE id > ?1 ORDER BY id")
        else {
            return vec![];
        };
        st.query_map([from], |r| {
            Ok((r.get::<_, String>(0)? == "user", r.get::<_, String>(1)?))
        })
        .map(|rows| rows.flatten().collect())
        .unwrap_or_default()
    }

        /// Скрыть всю текущую переписку. Строки остаются в базе ради статистики.
    pub fn clear_chat(&self) {
        let max: i64 = self
            .conn
            .query_row("SELECT COALESCE(MAX(id), 0) FROM messages", [], |r| r.get(0))
            .unwrap_or(0);
        let _ = self.set_setting("clear_id", &max.to_string());
    }
    
    // ---------- настройки ----------

    pub fn get_setting(&self, key: &str) -> Option<String> {
        self.conn
            .query_row("SELECT value FROM settings WHERE key = ?1", [key], |r| r.get(0))
            .optional()
            .ok()
            .flatten()
    }

    pub fn set_setting(&self, key: &str, value: &str) -> Result<()> {
        self.conn.execute(
            "INSERT INTO settings (key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![key, value],
        )?;
        Ok(())
    }

    // ---------- статистика ----------

    fn tokens_between(&self, from: i64, to: i64) -> u64 {
        self.conn
            .query_row(
                "SELECT COALESCE(SUM(tokens), 0) FROM messages WHERE ts >= ?1 AND ts < ?2",
                [from, to],
                |r| r.get::<_, i64>(0),
            )
            .unwrap_or(0) as u64
    }

    pub fn stats(&self) -> Stats {
        let t = now() + 1;
        let period = |days: i64| {
            let len = days * DAY;
            (
                self.tokens_between(t - len, t),
                self.tokens_between(t - 2 * len, t - len),
            )
        };
        let (day, prev_day) = period(1);
        let (week, prev_week) = period(7);
        let (month, prev_month) = period(30);

        let q = |sql: &str| -> u64 {
            self.conn.query_row(sql, [], |r| r.get::<_, i64>(0)).unwrap_or(0) as u64
        };

        Stats {
            messages_sent: q("SELECT COUNT(*) FROM messages WHERE role = 'user'"),
            day,
            prev_day,
            week,
            prev_week,
            month,
            prev_month,
            total: q("SELECT COALESCE(SUM(tokens), 0) FROM messages"),
            db_bytes: fs::metadata(self.dir.join("allma.db")).map(|m| m.len()).unwrap_or(0),
        }
    }

    // ---------- дневник ----------

    pub fn diary_path(&self) -> PathBuf {
        self.dir.join("diary.md")
    }

    pub fn read_diary(&self) -> String {
        fs::read_to_string(self.diary_path()).unwrap_or_default()
    }

    /// Дописать заметку в конец дневника.
    pub fn append_diary(&self, note: &str) -> std::io::Result<()> {
        let mut text = self.read_diary();
        if !text.ends_with('\n') {
            text.push('\n');
        }
        text.push_str(&format!("- {}\n", note.trim()));
        fs::write(self.diary_path(), text)
    }

    // ---------- промпт ----------

    pub fn prompt_path(&self) -> PathBuf {
        self.dir.join("prompt.txt")
    }

    pub fn read_prompt(&self) -> String {
        fs::read_to_string(self.prompt_path()).unwrap_or_else(|_| DEFAULT_PROMPT.into())
    }
}