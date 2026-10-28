pub mod dm;
pub mod gui;
pub mod play;
pub mod state;

/// Load a gitignored `.env` file from the working directory.
/// Existing environment variables win, so a shell export still overrides the file.
pub fn load_local_env() {
    if let Ok(text) = std::fs::read_to_string(".env") {
        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let Some((key, value)) = line.split_once('=') else {
                continue;
            };
            let key = key.trim();
            let value = value.trim().trim_matches('"').trim_matches('\'');
            if key.is_empty() || std::env::var(key).is_ok() {
                continue;
            }
            std::env::set_var(key, value);
        }
    }
    if std::env::var("OPENAI_API_KEY").is_err() {
        if let Ok(key) = std::env::var("XAI_API_KEY") {
            std::env::set_var("OPENAI_API_KEY", key);
        }
    }
}
