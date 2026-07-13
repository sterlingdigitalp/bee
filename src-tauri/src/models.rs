use futures_util::StreamExt;
use serde::Serialize;
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Emitter};
use tokio::{fs::File, io::AsyncWriteExt};

#[derive(Clone)]
pub struct ModelDefinition {
    pub id: &'static str,
    pub name: &'static str,
    pub detail: &'static str,
    pub size_mb: u32,
    pub filename: &'static str,
    pub url: &'static str,
    pub multilingual: bool,
    pub recommended: bool,
}
pub const MODELS:&[ModelDefinition]=&[
    ModelDefinition{id:"tiny-en",name:"Tiny (English)",detail:"Fastest · Basic accuracy",size_mb:75,filename:"ggml-tiny.en.bin",url:"https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-tiny.en.bin",multilingual:false,recommended:false},
    ModelDefinition{id:"base-en",name:"Base (English)",detail:"Fast · Good accuracy",size_mb:142,filename:"ggml-base.en.bin",url:"https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.en.bin",multilingual:false,recommended:true},
    ModelDefinition{id:"small-en",name:"Small (English)",detail:"Moderate · Better accuracy",size_mb:466,filename:"ggml-small.en.bin",url:"https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-small.en.bin",multilingual:false,recommended:false},
    ModelDefinition{id:"medium-en",name:"Medium (English)",detail:"Slower · Great accuracy",size_mb:1500,filename:"ggml-medium.en.bin",url:"https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-medium.en.bin",multilingual:false,recommended:false},
    ModelDefinition{id:"large-v3",name:"Large v3",detail:"Best accuracy · Multilingual",size_mb:3100,filename:"ggml-large-v3.bin",url:"https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-large-v3.bin",multilingual:true,recommended:false},
    ModelDefinition{id:"distil-large-v3",name:"Distil Large v3",detail:"Fast · Great accuracy",size_mb:1500,filename:"ggml-distil-large-v3.bin",url:"https://huggingface.co/distil-whisper/distil-large-v3-ggml/resolve/main/ggml-distil-large-v3.bin",multilingual:true,recommended:false},
    ModelDefinition{id:"parakeet-v3",name:"Parakeet V3",detail:"Fast multilingual · 25 languages",size_mb:671,filename:"parakeet-v3",url:"https://huggingface.co/s0me-0ne/parakeet-tdt-0.6b-v3-onnx/resolve/main",multilingual:true,recommended:false},
];

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelInfo {
    pub id: String,
    pub name: String,
    pub detail: String,
    pub size_mb: u32,
    pub downloaded: bool,
    pub active: bool,
    pub multilingual: bool,
    pub recommended: bool,
}
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct DownloadProgress {
    model_id: String,
    downloaded: u64,
    total: u64,
    percent: f32,
}

pub fn definition(id: &str) -> Option<&'static ModelDefinition> {
    MODELS.iter().find(|m| m.id == id)
}
pub fn model_path(dir: &Path, id: &str) -> Result<PathBuf, String> {
    let m = definition(id).ok_or("Unknown model")?;
    Ok(dir.join(m.filename))
}
pub fn list(dir: &Path, active: &str) -> Vec<ModelInfo> {
    MODELS
        .iter()
        .map(|m| {
            let path = dir.join(m.filename);
            let downloaded = if m.id == "parakeet-v3" {
                path.join("encoder-model.int8.onnx").exists()
            } else {
                path.exists()
            };
            ModelInfo {
                id: m.id.into(),
                name: m.name.into(),
                detail: m.detail.into(),
                size_mb: m.size_mb,
                downloaded,
                active: m.id == active && downloaded,
                multilingual: m.multilingual,
                recommended: m.recommended,
            }
        })
        .collect()
}

pub async fn download(app: &AppHandle, dir: &Path, id: &str) -> Result<(), String> {
    let model = definition(id).ok_or("Unknown model")?;
    tokio::fs::create_dir_all(dir)
        .await
        .map_err(|e| e.to_string())?;
    if id == "parakeet-v3" {
        return download_parakeet(app, dir, model).await;
    }
    let target = dir.join(model.filename);
    let partial = target.with_extension("download");
    let response = reqwest::Client::new()
        .get(model.url)
        .send()
        .await
        .map_err(|e| e.to_string())?
        .error_for_status()
        .map_err(|e| e.to_string())?;
    let total = response
        .content_length()
        .unwrap_or((model.size_mb as u64) * 1_000_000);
    let mut stream = response.bytes_stream();
    let mut file = File::create(&partial).await.map_err(|e| e.to_string())?;
    let mut downloaded = 0u64;
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| e.to_string())?;
        file.write_all(&chunk).await.map_err(|e| e.to_string())?;
        downloaded += chunk.len() as u64;
        let _ = app.emit(
            "model-download",
            DownloadProgress {
                model_id: id.into(),
                downloaded,
                total,
                percent: (downloaded as f32 / total.max(1) as f32) * 100.,
            },
        );
    }
    file.flush().await.map_err(|e| e.to_string())?;
    drop(file);
    tokio::fs::rename(partial, target)
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

async fn download_parakeet(
    app: &AppHandle,
    dir: &Path,
    model: &ModelDefinition,
) -> Result<(), String> {
    let target = dir.join(model.filename);
    let partial = dir.join("parakeet-v3.download");
    if partial.exists() {
        tokio::fs::remove_dir_all(&partial)
            .await
            .map_err(|e| e.to_string())?;
    }
    tokio::fs::create_dir_all(&partial)
        .await
        .map_err(|e| e.to_string())?;
    let files = [
        "encoder-model.int8.onnx",
        "decoder_joint-model.int8.onnx",
        "nemo128.onnx",
        "vocab.txt",
    ];
    let client = reqwest::Client::new();
    let mut complete = 0u64;
    for (index, name) in files.iter().enumerate() {
        let url = format!("{}/{}?download=true", model.url, name);
        let response = client
            .get(url)
            .send()
            .await
            .map_err(|e| e.to_string())?
            .error_for_status()
            .map_err(|e| e.to_string())?;
        let file_total = response.content_length().unwrap_or(1);
        let mut stream = response.bytes_stream();
        let mut file = File::create(partial.join(name))
            .await
            .map_err(|e| e.to_string())?;
        let mut file_done = 0u64;
        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|e| e.to_string())?;
            file.write_all(&chunk).await.map_err(|e| e.to_string())?;
            file_done += chunk.len() as u64;
            let percent = ((index as f32 + file_done as f32 / file_total.max(1) as f32)
                / files.len() as f32)
                * 100.0;
            let _ = app.emit(
                "model-download",
                DownloadProgress {
                    model_id: model.id.into(),
                    downloaded: complete + file_done,
                    total: model.size_mb as u64 * 1_000_000,
                    percent: percent.min(99.0),
                },
            );
        }
        file.flush().await.map_err(|e| e.to_string())?;
        complete += file_total;
    }
    if target.exists() {
        tokio::fs::remove_dir_all(&target)
            .await
            .map_err(|e| e.to_string())?;
    }
    tokio::fs::rename(partial, target)
        .await
        .map_err(|e| e.to_string())?;
    let _ = app.emit(
        "model-download",
        DownloadProgress {
            model_id: model.id.into(),
            downloaded: complete,
            total: complete,
            percent: 100.0,
        },
    );
    Ok(())
}
pub async fn delete(dir: &Path, id: &str) -> Result<(), String> {
    let path = model_path(dir, id)?;
    if path.is_dir() {
        tokio::fs::remove_dir_all(path)
            .await
            .map_err(|e| e.to_string())?
    } else if path.exists() {
        tokio::fs::remove_file(path)
            .await
            .map_err(|e| e.to_string())?
    }
    Ok(())
}
