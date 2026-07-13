use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppConfig {
    pub transcription_mode: String,
    pub model: String,
    pub cloud_language: String,
    pub push_to_talk_key: String,
    pub toggle_key: Option<String>,
    #[serde(default)]
    pub shortcuts_paused: bool,
    #[serde(default)]
    pub dismissed_suggestions: Vec<String>,
    #[serde(default)]
    pub close_notice_seen: bool,
    pub recording_mode: String,
    pub preferred_input_device: Option<String>,
    pub fallback_input_device: Option<String>,
    pub input_gain: f32,
    pub auto_punctuation: bool,
    pub remove_fillers: bool,
    pub copy_to_clipboard: bool,
    pub auto_enhance_prompt: bool,
    pub custom_instructions: String,
    pub dictionary_enabled: bool,
    pub recording_sound_enabled: bool,
    pub auto_hide_widget: bool,
    pub show_widget: bool,
    pub follow_cursor: bool,
    pub lock_widget_position: bool,
    pub theme: String,
    pub launch_at_login: bool,
    pub onboarding_complete: bool,
    pub groq_api_key_configured: bool,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            transcription_mode: "local".into(),
            model: "base-en".into(),
            cloud_language: "auto".into(),
            push_to_talk_key: recommended_hotkey(),
            toggle_key: None,
            shortcuts_paused: false,
            dismissed_suggestions: Vec::new(),
            close_notice_seen: false,
            recording_mode: "push-to-talk".into(),
            preferred_input_device: None,
            fallback_input_device: None,
            input_gain: 1.0,
            auto_punctuation: true,
            remove_fillers: true,
            copy_to_clipboard: false,
            auto_enhance_prompt: false,
            custom_instructions: String::new(),
            dictionary_enabled: true,
            recording_sound_enabled: true,
            auto_hide_widget: false,
            show_widget: true,
            follow_cursor: true,
            lock_widget_position: false,
            theme: "black".into(),
            launch_at_login: false,
            onboarding_complete: false,
            groq_api_key_configured: false,
        }
    }
}

#[cfg(target_os = "macos")]
fn recommended_hotkey() -> String {
    "Fn / Globe".into()
}
#[cfg(target_os = "windows")]
fn recommended_hotkey() -> String {
    "Ctrl + Win".into()
}
#[cfg(target_os = "linux")]
fn recommended_hotkey() -> String {
    "Ctrl + Alt".into()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryItem {
    pub id: String,
    pub text: String,
    pub raw_text: String,
    pub timestamp: i64,
    pub word_count: usize,
    pub duration_seconds: f32,
    pub transcription_ms: u128,
    pub model: String,
    pub source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DictionaryEntry {
    pub id: String,
    pub original: String,
    pub replacement: String,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct PersistentData {
    pub config: AppConfig,
    pub history: Vec<HistoryItem>,
    pub dictionary: Vec<DictionaryEntry>,
}

pub struct Store {
    pub path: PathBuf,
    pub data: PersistentData,
}
impl Store {
    pub fn load(dir: &Path) -> Result<Self, String> {
        fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        let path = dir.join("bee.json");
        let data = if path.exists() {
            serde_json::from_slice(&fs::read(&path).map_err(|e| e.to_string())?).unwrap_or_default()
        } else {
            PersistentData::default()
        };
        Ok(Self { path, data })
    }
    pub fn save(&self) -> Result<(), String> {
        let tmp = self.path.with_extension("json.tmp");
        fs::write(
            &tmp,
            serde_json::to_vec_pretty(&self.data).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
        fs::rename(tmp, &self.path).map_err(|e| e.to_string())
    }
    pub fn upsert_dictionary(&mut self, original: String, replacement: String) -> DictionaryEntry {
        if let Some(item) = self
            .data
            .dictionary
            .iter_mut()
            .find(|e| e.original.eq_ignore_ascii_case(&original))
        {
            item.replacement = replacement;
            return item.clone();
        }
        let item = DictionaryEntry {
            id: Uuid::new_v4().to_string(),
            original,
            replacement,
            created_at: chrono::Utc::now().timestamp_millis(),
        };
        self.data.dictionary.push(item.clone());
        item
    }
}

pub fn merge_config(config: &mut AppConfig, patch: serde_json::Value) -> Result<(), String> {
    let mut current = serde_json::to_value(&*config).map_err(|e| e.to_string())?;
    if let (Some(dst), Some(src)) = (current.as_object_mut(), patch.as_object()) {
        for (k, v) in src {
            dst.insert(k.clone(), v.clone());
        }
    }
    *config = serde_json::from_value(current).map_err(|e| e.to_string())?;
    Ok(())
}

pub fn apply_dictionary(mut text: String, entries: &[DictionaryEntry]) -> String {
    for entry in entries {
        text = replace_case_insensitive(&text, &entry.original, &entry.replacement);
    }
    text
}
fn replace_case_insensitive(haystack: &str, needle: &str, replacement: &str) -> String {
    if needle.is_empty() {
        return haystack.into();
    }
    regex::RegexBuilder::new(&regex::escape(needle))
        .case_insensitive(true)
        .build()
        .map(|pattern| {
            pattern
                .replace_all(haystack, |_: &regex::Captures<'_>| replacement.to_string())
                .into_owned()
        })
        .unwrap_or_else(|_| haystack.into())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn dictionary_replacements_are_case_insensitive() {
        let entries = vec![DictionaryEntry {
            id: "1".into(),
            original: "next js".into(),
            replacement: "Next.js".into(),
            created_at: 0,
        }];
        assert_eq!(
            apply_dictionary("Use NEXT JS today".into(), &entries),
            "Use Next.js today"
        );
        assert_eq!(replace_case_insensitive("CAFÉ", "café", "$HOME"), "$HOME");
    }
    #[test]
    fn cleanup_removes_fillers_and_adds_punctuation() {
        assert_eq!(
            clean_transcript("um, ship the patch", true, true),
            "Ship the patch."
        );
        assert_eq!(
            clean_transcript("Uh I like Rust", true, true),
            "I like Rust."
        );
    }
    #[test]
    fn config_patch_preserves_unmentioned_values() {
        let mut config = AppConfig::default();
        merge_config(
            &mut config,
            serde_json::json!({"theme":"light","copyToClipboard":true}),
        )
        .unwrap();
        assert_eq!(config.theme, "light");
        assert!(config.copy_to_clipboard);
        assert!(config.dictionary_enabled);
    }
}

pub fn clean_transcript(text: &str, remove_fillers: bool, auto_punctuation: bool) -> String {
    let mut out = text.trim().split_whitespace().collect::<Vec<_>>().join(" ");
    if remove_fillers {
        let fillers = regex::Regex::new(r"(?i)\b(?:um+|uh+|erm+|you know|i mean)\b[,;:\s]*")
            .expect("valid filler pattern");
        out = fillers.replace_all(&out, "").trim().to_string();
        out = out.split_whitespace().collect::<Vec<_>>().join(" ");
    }
    if auto_punctuation && !out.is_empty() {
        let mut chars = out.chars();
        out = chars.next().unwrap().to_uppercase().collect::<String>() + chars.as_str();
        if !matches!(out.chars().last(), Some('.' | '!' | '?')) {
            out.push('.')
        }
    }
    out
}
