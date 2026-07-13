use crate::storage::AppConfig;
use std::path::Path;
use transcribe_rs::onnx::{
    parakeet::{ParakeetModel, ParakeetParams},
    Quantization,
};
use transcribe_rs::whisper_cpp::{WhisperEngine, WhisperInferenceParams};

pub enum LoadedEngine {
    Whisper(WhisperEngine),
    Parakeet(ParakeetModel),
}
pub struct EngineCache {
    pub model_id: String,
    engine: LoadedEngine,
}

impl EngineCache {
    pub fn load(model_id: &str, model: &Path) -> Result<Self, String> {
        let engine = if model_id == "parakeet-v3" {
            LoadedEngine::Parakeet(
                ParakeetModel::load(model, &Quantization::Int8).map_err(|e| e.to_string())?,
            )
        } else {
            LoadedEngine::Whisper(WhisperEngine::load(model).map_err(|e| e.to_string())?)
        };
        Ok(Self {
            model_id: model_id.into(),
            engine,
        })
    }
    pub fn transcribe(&mut self, samples: &[f32], config: &AppConfig) -> Result<String, String> {
        match &mut self.engine {
            LoadedEngine::Parakeet(engine) => engine
                .transcribe_with(samples, &ParakeetParams::default())
                .map(|r| r.text)
                .map_err(|e| e.to_string()),
            LoadedEngine::Whisper(engine) => {
                let language = if self.model_id.ends_with("-en") {
                    Some("en".to_string())
                } else if config.cloud_language == "auto" {
                    None
                } else {
                    Some(config.cloud_language.clone())
                };
                let prompt = if config.custom_instructions.trim().is_empty() {
                    None
                } else {
                    Some(config.custom_instructions.clone())
                };
                engine
                    .transcribe_with(
                        samples,
                        &WhisperInferenceParams {
                            language,
                            initial_prompt: prompt,
                            ..Default::default()
                        },
                    )
                    .map(|r| r.text)
                    .map_err(|e| e.to_string())
            }
        }
    }
}

pub async fn cloud(samples: &[f32], config: &AppConfig) -> Result<String, String> {
    let key = keyring::Entry::new("Bee", "groq-api-key")
        .map_err(|e| e.to_string())?
        .get_password()
        .map_err(|_| "No Groq API key configured".to_string())?;
    let mut wav = std::io::Cursor::new(Vec::new());
    {
        let spec = hound::WavSpec {
            channels: 1,
            sample_rate: 16000,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        let mut writer = hound::WavWriter::new(&mut wav, spec).map_err(|e| e.to_string())?;
        for s in samples {
            writer
                .write_sample((s.clamp(-1., 1.) * i16::MAX as f32) as i16)
                .map_err(|e| e.to_string())?
        }
        writer.finalize().map_err(|e| e.to_string())?
    }
    let file = reqwest::multipart::Part::bytes(wav.into_inner())
        .file_name("recording.wav")
        .mime_str("audio/wav")
        .map_err(|e| e.to_string())?;
    let mut form = reqwest::multipart::Form::new()
        .part("file", file)
        .text("model", "whisper-large-v3-turbo")
        .text("response_format", "json");
    if config.cloud_language != "auto" {
        form = form.text("language", config.cloud_language.clone())
    }
    let value: reqwest::Response = reqwest::Client::new()
        .post("https://api.groq.com/openai/v1/audio/transcriptions")
        .bearer_auth(key)
        .multipart(form)
        .send()
        .await
        .map_err(|e| e.to_string())?
        .error_for_status()
        .map_err(|e| e.to_string())?;
    let json: serde_json::Value = value.json().await.map_err(|e| e.to_string())?;
    json.get("text")
        .and_then(|x| x.as_str())
        .map(str::to_owned)
        .ok_or("Cloud response contained no text".into())
}

pub async fn enhance(text: &str, instructions: &str, mode: &str) -> Result<String, String> {
    let key = keyring::Entry::new("Bee", "groq-api-key")
        .map_err(|e| e.to_string())?
        .get_password()
        .map_err(|_| "No Groq API key configured".to_string())?;
    let system = if mode == "enhance" {
        "Rewrite the user's dictated text as a clear, complete prompt for an AI coding agent. Return only the rewritten prompt."
    } else {
        "Polish the user's dictated text for clarity and grammar without changing meaning. Return only the polished text."
    };
    let system = if instructions.trim().is_empty() {
        system.to_string()
    } else {
        format!("{system}\nUser preferences: {instructions}")
    };
    let body = serde_json::json!({"model":"llama-3.3-70b-versatile","temperature":0.2,"messages":[{"role":"system","content":system},{"role":"user","content":text}]});
    let json: serde_json::Value = reqwest::Client::new()
        .post("https://api.groq.com/openai/v1/chat/completions")
        .bearer_auth(key)
        .json(&body)
        .send()
        .await
        .map_err(|e| e.to_string())?
        .error_for_status()
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())?;
    json.pointer("/choices/0/message/content")
        .and_then(|x| x.as_str())
        .map(|s| s.trim().to_string())
        .ok_or("Rewrite returned no text".into())
}
