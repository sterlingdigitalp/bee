mod audio;
#[cfg(target_os = "macos")]
mod macos_fn;
mod models;
mod storage;
mod transcription;

use audio::{AudioDevice, Recorder};
use enigo::{Direction, Enigo, Key, Keyboard, Mouse, Settings as EnigoSettings};
use serde::Serialize;
use std::{
    path::PathBuf,
    sync::Mutex,
    time::{Duration, Instant},
};
use storage::{AppConfig, DictionaryEntry, HistoryItem, Store};
use tauri::{AppHandle, Emitter, Manager, State, WebviewWindow};
use tauri_plugin_autostart::ManagerExt as AutoStartExt;
use tauri_plugin_clipboard_manager::ClipboardExt;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};
use tauri_plugin_opener::OpenerExt;
use uuid::Uuid;

pub struct AppState {
    store: Mutex<Store>,
    recorder: Mutex<Option<Recorder>>,
    engine: Mutex<Option<transcription::EngineCache>>,
    models_dir: PathBuf,
}
unsafe impl Sync for AppState {}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Stats {
    total_words: usize,
    total_seconds: f32,
    total_sessions: usize,
    average_wpm: usize,
    today_words: usize,
    week_words: usize,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct RuntimeSnapshot {
    config: AppConfig,
    models: Vec<models::ModelInfo>,
    history: Vec<HistoryItem>,
    dictionary: Vec<DictionaryEntry>,
    stats: Stats,
    audio_devices: Vec<AudioDevice>,
    version: String,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct PermissionStatus {
    microphone: bool,
    input_monitoring: bool,
    accessibility: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct UpdateInfo {
    current_version: String,
    latest_version: String,
    available: bool,
    notes: String,
    download_url: Option<String>,
}

#[derive(serde::Deserialize)]
struct UpdateManifest {
    version: String,
    #[serde(default)]
    notes: String,
    url: Option<String>,
}

fn stats(items: &[HistoryItem]) -> Stats {
    let now = chrono::Utc::now().timestamp_millis();
    let total_words = items.iter().map(|x| x.word_count).sum();
    let total_seconds = items.iter().map(|x| x.duration_seconds).sum::<f32>();
    Stats {
        total_words,
        total_seconds,
        total_sessions: items.len(),
        average_wpm: if total_seconds > 0.0 {
            ((total_words as f32) / (total_seconds / 60.0)) as usize
        } else {
            0
        },
        today_words: items
            .iter()
            .filter(|x| now - x.timestamp < 86_400_000)
            .map(|x| x.word_count)
            .sum(),
        week_words: items
            .iter()
            .filter(|x| now - x.timestamp < 604_800_000)
            .map(|x| x.word_count)
            .sum(),
    }
}

#[tauri::command]
fn get_snapshot(app: AppHandle, state: State<AppState>) -> Result<RuntimeSnapshot, String> {
    let store = state.store.lock().map_err(|_| "State lock failed")?;
    let config = store.data.config.clone();
    Ok(RuntimeSnapshot {
        models: models::list(&state.models_dir, &config.model),
        stats: stats(&store.data.history),
        history: store.data.history.clone(),
        dictionary: store.data.dictionary.clone(),
        audio_devices: audio::list_devices(),
        version: app.package_info().version.to_string(),
        config,
    })
}

#[tauri::command]
fn update_config(
    app: AppHandle,
    state: State<AppState>,
    patch: serde_json::Value,
) -> Result<AppConfig, String> {
    let mut store = state.store.lock().map_err(|_| "State lock failed")?;
    let previous = store.data.config.clone();
    storage::merge_config(&mut store.data.config, patch)?;
    if store.data.config.launch_at_login {
        let _ = app.autolaunch().enable();
    } else {
        let _ = app.autolaunch().disable();
    }
    if let Some(widget) = app.get_webview_window("widget") {
        if store.data.config.show_widget && !store.data.config.auto_hide_widget {
            let _ = widget.show();
        } else if !store.data.config.show_widget || store.data.config.auto_hide_widget {
            let _ = widget.hide();
        }
    }
    store.save()?;
    let config = store.data.config.clone();
    drop(store);
    refresh_shortcuts(&app, &previous, &config);
    let _ = app.emit("config-changed", &config);
    Ok(config)
}

fn portable_shortcut(value: &str) -> Option<String> {
    if value.contains("Fn") || value.trim().is_empty() {
        return None;
    }
    let normalized = value
        .replace('⌘', "Command")
        .replace('⌃', "Control")
        .replace('⌥', "Alt")
        .replace('⇧', "Shift")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join("+");
    (!normalized.is_empty()).then_some(normalized)
}

fn configured_shortcuts(config: &AppConfig) -> Vec<String> {
    let mut shortcuts = Vec::new();
    if let Some(value) = portable_shortcut(&config.push_to_talk_key) {
        shortcuts.push(value);
    }
    if let Some(value) = config.toggle_key.as_deref().and_then(portable_shortcut) {
        if !shortcuts.contains(&value) {
            shortcuts.push(value);
        }
    }
    shortcuts
}

fn refresh_shortcuts(app: &AppHandle, previous: &AppConfig, next: &AppConfig) {
    let manager = app.global_shortcut();
    for value in configured_shortcuts(previous) {
        if value != "CommandOrControl+Shift+Space" {
            let _ = manager.unregister(value.as_str());
        }
    }
    if !next.shortcuts_paused {
        for value in configured_shortcuts(next) {
            if value != "CommandOrControl+Shift+Space" {
                let _ = manager.register(value.as_str());
            }
        }
    }
}

fn emit_state(app: &AppHandle, value: &str) {
    let _ = app.emit("recording-state", value);
}
async fn begin_recording(app: AppHandle) -> Result<(), String> {
    let state = app.state::<AppState>();
    if state
        .recorder
        .lock()
        .map_err(|_| "Recorder lock failed")?
        .is_some()
    {
        return Ok(());
    }
    let (config, show) = {
        let s = state.store.lock().map_err(|_| "State lock failed")?;
        (s.data.config.clone(), s.data.config.show_widget)
    };
    let recorder = match audio::start(
        app.clone(),
        config.preferred_input_device.as_deref(),
        config.fallback_input_device.as_deref(),
        config.input_gain,
    ) {
        Ok(recorder) => recorder,
        Err(error) => {
            emit_state(&app, "error");
            let _ = app.emit("recording-error", &error);
            return Err(error);
        }
    };
    *state.recorder.lock().map_err(|_| "Recorder lock failed")? = Some(recorder);
    if config.recording_sound_enabled {
        audio::play_chime(true);
    }
    if show {
        if let Some(widget) = app.get_webview_window("widget") {
            if config.follow_cursor && !config.lock_widget_position {
                if let Ok(enigo) = Enigo::new(&EnigoSettings::default()) {
                    if let Ok((cursor_x, cursor_y)) = enigo.location() {
                        let x = (cursor_x - 110).max(8);
                        let y = (cursor_y + 24).max(8);
                        let _ = widget.set_position(tauri::LogicalPosition::new(x, y));
                    }
                }
            }
            let _ = widget.show();
        }
    }
    emit_state(&app, "listening");
    Ok(())
}

async fn end_recording(app: AppHandle) -> Result<Option<HistoryItem>, String> {
    let state = app.state::<AppState>();
    let recorder = state
        .recorder
        .lock()
        .map_err(|_| "Recorder lock failed")?
        .take();
    let Some(recorder) = recorder else {
        return Ok(None);
    };
    emit_state(&app, "processing");
    let (samples, duration_seconds) = audio::finish(recorder);
    if samples.len() < 1600 {
        let message = "No speech detected";
        emit_state(&app, "error");
        let _ = app.emit("recording-error", message);
        return Err(message.into());
    }
    let peak = samples
        .iter()
        .fold(0.0_f32, |value, sample| value.max(sample.abs()));
    let rms = (samples.iter().map(|sample| sample * sample).sum::<f32>()
        / samples.len().max(1) as f32)
        .sqrt();
    if peak < 0.01 && rms < 0.0015 {
        let message = "No speech detected. Check the selected microphone or input gain.";
        emit_state(&app, "error");
        let _ = app.emit("recording-error", message);
        return Err(message.into());
    }
    let (config, dictionary) = {
        let s = state.store.lock().map_err(|_| "State lock failed")?;
        (s.data.config.clone(), s.data.dictionary.clone())
    };
    if config.recording_sound_enabled {
        audio::play_chime(false);
    }
    let started = Instant::now();
    let transcription_result = if config.transcription_mode == "cloud" {
        transcription::cloud(&samples, &config).await
    } else {
        match models::model_path(&state.models_dir, &config.model) {
            Err(error) => Err(error),
            Ok(path) if !path.exists() => Err(format!("Model {} is not downloaded", config.model)),
            Ok(path) => {
                let cfg = config.clone();
                let app_for_model = app.clone();
                tokio::task::spawn_blocking(move || {
                    let state = app_for_model.state::<AppState>();
                    let mut cache = state
                        .engine
                        .lock()
                        .map_err(|_| "Model cache lock failed".to_string())?;
                    if cache
                        .as_ref()
                        .is_none_or(|loaded| loaded.model_id != cfg.model)
                    {
                        *cache = Some(transcription::EngineCache::load(&cfg.model, &path)?);
                    }
                    cache
                        .as_mut()
                        .ok_or("Model cache unavailable".to_string())?
                        .transcribe(&samples, &cfg)
                })
                .await
                .map_err(|e| e.to_string())
                .and_then(|result| result)
            }
        }
    };
    let raw_text = match transcription_result {
        Ok(text) => text,
        Err(error) => {
            emit_state(&app, "error");
            let _ = app.emit("recording-error", &error);
            return Err(error);
        }
    };
    let mut text =
        storage::clean_transcript(&raw_text, config.remove_fillers, config.auto_punctuation);
    if text.trim().is_empty() {
        let message = "No speech was recognized";
        emit_state(&app, "error");
        let _ = app.emit("recording-error", message);
        return Err(message.into());
    }
    if config.dictionary_enabled {
        text = storage::apply_dictionary(text, &dictionary)
    }
    if config.auto_enhance_prompt {
        if let Ok(next) =
            transcription::enhance(&text, &config.custom_instructions, "enhance").await
        {
            text = next
        }
    }
    let item = HistoryItem {
        id: Uuid::new_v4().to_string(),
        word_count: text.split_whitespace().count(),
        text: text.clone(),
        raw_text,
        timestamp: chrono::Utc::now().timestamp_millis(),
        duration_seconds,
        transcription_ms: started.elapsed().as_millis(),
        model: models::definition(&config.model)
            .map(|m| m.name)
            .unwrap_or("Cloud Whisper")
            .into(),
        source: config.transcription_mode.clone(),
    };
    {
        let mut s = state.store.lock().map_err(|_| "State lock failed")?;
        s.data.history.insert(0, item.clone());
        s.save()?
    }
    app.clipboard()
        .write_text(text.clone())
        .map_err(|e| e.to_string())?;
    let _ = app.emit("data-changed", ());
    if !config.copy_to_clipboard {
        tokio::time::sleep(Duration::from_millis(70)).await;
        if let Err(error) = paste_shortcut() {
            emit_state(&app, "error");
            let message = format!(
                "Text was copied, but could not be pasted. Grant Accessibility permission: {error}"
            );
            let _ = app.emit("recording-error", &message);
            return Err(message);
        }
    }
    emit_state(&app, "success");
    if config.auto_hide_widget {
        if let Some(widget) = app.get_webview_window("widget") {
            let app2 = app.clone();
            tauri::async_runtime::spawn(async move {
                tokio::time::sleep(Duration::from_millis(1200)).await;
                let _ = widget.hide();
                let _ = app2.emit("recording-state", "idle");
            });
        }
    } else {
        let app2 = app.clone();
        tauri::async_runtime::spawn(async move {
            tokio::time::sleep(Duration::from_millis(1200)).await;
            emit_state(&app2, "idle");
        });
    }
    Ok(Some(item))
}
fn paste_shortcut() -> Result<(), String> {
    let mut enigo = Enigo::new(&EnigoSettings::default()).map_err(|e| e.to_string())?;
    #[cfg(target_os = "macos")]
    let modifier = Key::Meta;
    #[cfg(not(target_os = "macos"))]
    let modifier = Key::Control;
    enigo
        .key(modifier, Direction::Press)
        .map_err(|e| e.to_string())?;
    let clicked = enigo
        .key(Key::Unicode('v'), Direction::Click)
        .map_err(|e| e.to_string());
    let _ = enigo.key(modifier, Direction::Release);
    clicked
}

#[tauri::command]
async fn start_recording(app: AppHandle) -> Result<(), String> {
    begin_recording(app).await
}
#[tauri::command]
async fn stop_recording(app: AppHandle) -> Result<Option<HistoryItem>, String> {
    end_recording(app).await
}
#[tauri::command]
fn cancel_recording(app: AppHandle, state: State<AppState>) -> Result<(), String> {
    if let Some(rec) = state
        .recorder
        .lock()
        .map_err(|_| "Recorder lock failed")?
        .take()
    {
        drop(rec.stream)
    }
    emit_state(&app, "idle");
    Ok(())
}
#[tauri::command]
fn select_model(app: AppHandle, state: State<AppState>, model_id: String) -> Result<(), String> {
    let path = models::model_path(&state.models_dir, &model_id)?;
    if !path.exists() {
        return Err("Download the model first".into());
    }
    let mut s = state.store.lock().map_err(|_| "State lock failed")?;
    s.data.config.model = model_id;
    s.data.config.transcription_mode = "local".into();
    s.save()?;
    *state.engine.lock().map_err(|_| "Model cache lock failed")? = None;
    let _ = app.emit("data-changed", ());
    Ok(())
}
#[tauri::command]
async fn download_model(
    app: AppHandle,
    state: State<'_, AppState>,
    model_id: String,
) -> Result<(), String> {
    models::download(&app, &state.models_dir, &model_id).await?;
    let _ = app.emit("data-changed", ());
    Ok(())
}
#[tauri::command]
async fn delete_model(
    app: AppHandle,
    state: State<'_, AppState>,
    model_id: String,
) -> Result<(), String> {
    models::delete(&state.models_dir, &model_id).await?;
    let _ = app.emit("data-changed", ());
    Ok(())
}
#[tauri::command]
fn delete_history(app: AppHandle, state: State<AppState>, id: String) -> Result<(), String> {
    let mut s = state.store.lock().map_err(|_| "State lock failed")?;
    s.data.history.retain(|x| x.id != id);
    s.save()?;
    let _ = app.emit("data-changed", ());
    Ok(())
}
#[tauri::command]
fn clear_history(app: AppHandle, state: State<AppState>) -> Result<(), String> {
    let mut s = state.store.lock().map_err(|_| "State lock failed")?;
    s.data.history.clear();
    s.save()?;
    let _ = app.emit("data-changed", ());
    Ok(())
}
#[tauri::command]
fn export_history(app: AppHandle, state: State<AppState>) -> Result<String, String> {
    let s = state.store.lock().map_err(|_| "State lock failed")?;
    let contents = serde_json::to_string_pretty(&s.data.history).map_err(|e| e.to_string())?;
    let filename = format!(
        "Bee History {}.json",
        chrono::Local::now().format("%Y-%m-%d")
    );
    let directory = app
        .path()
        .download_dir()
        .or_else(|_| app.path().document_dir())
        .map_err(|e| e.to_string())?;
    let path = directory.join(filename);
    std::fs::write(&path, contents).map_err(|e| e.to_string())?;
    Ok(path.to_string_lossy().into_owned())
}
#[tauri::command]
fn upsert_dictionary(
    app: AppHandle,
    state: State<AppState>,
    original: String,
    replacement: String,
) -> Result<DictionaryEntry, String> {
    let mut s = state.store.lock().map_err(|_| "State lock failed")?;
    let item = s.upsert_dictionary(original, replacement);
    s.save()?;
    let _ = app.emit("data-changed", ());
    Ok(item)
}
#[tauri::command]
fn delete_dictionary(app: AppHandle, state: State<AppState>, id: String) -> Result<(), String> {
    let mut s = state.store.lock().map_err(|_| "State lock failed")?;
    s.data.dictionary.retain(|x| x.id != id);
    s.save()?;
    let _ = app.emit("data-changed", ());
    Ok(())
}
#[tauri::command]
fn copy_text(app: AppHandle, text: String) -> Result<(), String> {
    app.clipboard().write_text(text).map_err(|e| e.to_string())
}
#[tauri::command]
async fn polish_transcription(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
    mode: String,
) -> Result<HistoryItem, String> {
    let (item, instructions) = {
        let s = state.store.lock().map_err(|_| "State lock failed")?;
        (
            s.data
                .history
                .iter()
                .find(|x| x.id == id)
                .cloned()
                .ok_or("Transcription not found")?,
            s.data.config.custom_instructions.clone(),
        )
    };
    let rewritten = transcription::enhance(&item.text, &instructions, &mode).await?;
    let mut s = state.store.lock().map_err(|_| "State lock failed")?;
    let found = s
        .data
        .history
        .iter_mut()
        .find(|x| x.id == id)
        .ok_or("Transcription not found")?;
    found.text = rewritten;
    found.word_count = found.text.split_whitespace().count();
    let out = found.clone();
    s.save()?;
    let _ = app.emit("data-changed", ());
    Ok(out)
}
#[tauri::command]
fn set_groq_api_key(app: AppHandle, state: State<AppState>, key: String) -> Result<(), String> {
    let entry = keyring::Entry::new("Bee", "groq-api-key").map_err(|e| e.to_string())?;
    if key.trim().is_empty() {
        let _ = entry.delete_credential();
    } else {
        entry.set_password(key.trim()).map_err(|e| e.to_string())?
    }
    let mut s = state.store.lock().map_err(|_| "State lock failed")?;
    s.data.config.groq_api_key_configured = !key.trim().is_empty();
    s.save()?;
    let _ = app.emit("data-changed", ());
    Ok(())
}
#[tauri::command]
fn show_dashboard(app: AppHandle) -> Result<(), String> {
    let w = app
        .get_webview_window("dashboard")
        .ok_or("Dashboard not found")?;
    w.show().map_err(|e| e.to_string())?;
    w.set_focus().map_err(|e| e.to_string())
}

#[tauri::command]
fn acknowledge_close_notice(app: AppHandle, state: State<AppState>) -> Result<(), String> {
    let mut store = state.store.lock().map_err(|_| "State lock failed")?;
    store.data.config.close_notice_seen = true;
    store.save()?;
    drop(store);
    if let Some(window) = app.get_webview_window("dashboard") {
        window.hide().map_err(|e| e.to_string())?;
    }
    Ok(())
}
#[tauri::command]
fn quit_app(app: AppHandle) {
    app.exit(0)
}

#[tauri::command]
fn check_permissions() -> PermissionStatus {
    #[cfg(target_os = "macos")]
    {
        PermissionStatus {
            microphone: !audio::list_devices().is_empty(),
            input_monitoring: macos_fn::input_monitoring_allowed(),
            accessibility: macos_fn::accessibility_allowed(),
        }
    }
    #[cfg(not(target_os = "macos"))]
    {
        PermissionStatus {
            microphone: !audio::list_devices().is_empty(),
            input_monitoring: true,
            accessibility: true,
        }
    }
}

#[tauri::command]
async fn request_permissions(app: AppHandle) -> PermissionStatus {
    #[cfg(target_os = "macos")]
    {
        let _ = macos_fn::request_input_monitoring();
        let config = app
            .state::<AppState>()
            .store
            .lock()
            .ok()
            .map(|s| s.data.config.clone());
        if let Some(config) = config {
            if let Ok(recorder) = audio::start(
                app.clone(),
                config.preferred_input_device.as_deref(),
                config.fallback_input_device.as_deref(),
                config.input_gain,
            ) {
                drop(recorder.stream);
            }
        }
        if !macos_fn::accessibility_allowed() {
            let _ = app.opener().open_url(
                "x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility",
                None::<&str>,
            );
        }
        check_permissions()
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = app;
        check_permissions()
    }
}

#[tauri::command]
async fn check_for_updates(app: AppHandle) -> Result<UpdateInfo, String> {
    let current = app.package_info().version.to_string();
    let Some(feed) = option_env!("BEE_UPDATE_FEED") else {
        return Ok(UpdateInfo {
            current_version: current.clone(),
            latest_version: current,
            available: false,
            notes: "This local build has no release feed configured.".into(),
            download_url: None,
        });
    };
    let manifest: UpdateManifest = reqwest::Client::new()
        .get(feed)
        .send()
        .await
        .map_err(|e| e.to_string())?
        .error_for_status()
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())?;
    let available = semver::Version::parse(&manifest.version).map_err(|e| e.to_string())?
        > semver::Version::parse(&current).map_err(|e| e.to_string())?;
    Ok(UpdateInfo {
        current_version: current,
        latest_version: manifest.version,
        available,
        notes: manifest.notes,
        download_url: manifest.url,
    })
}

fn position_widget(window: &WebviewWindow) {
    if let Ok(Some(monitor)) = window.primary_monitor() {
        let size = monitor.size();
        let scale = monitor.scale_factor();
        let physical = window
            .outer_size()
            .unwrap_or(tauri::PhysicalSize::new(220, 50));
        let x = (size.width.saturating_sub(physical.width)) / 2;
        let y = size.height.saturating_sub(physical.height + 32);
        let _ = window.set_position(tauri::PhysicalPosition::new(x, y));
        let _ = scale;
    }
}

pub fn run() {
    let shortcut_plugin = tauri_plugin_global_shortcut::Builder::new()
        .with_shortcuts(["CommandOrControl+Shift+Space"])
        .expect("shortcut config")
        .with_handler(|app, shortcut, event| {
            let config = app
                .state::<AppState>()
                .store
                .lock()
                .ok()
                .map(|store| store.data.config.clone());
            let Some(config) = config else { return };
            if config.shortcuts_paused {
                return;
            }
            let is_toggle = config
                .toggle_key
                .as_deref()
                .and_then(portable_shortcut)
                .and_then(|value| value.parse::<Shortcut>().ok())
                .is_some_and(|toggle| toggle.id() == shortcut.id());
            let app = app.clone();
            tauri::async_runtime::spawn(async move {
                if is_toggle {
                    if event.state != ShortcutState::Pressed {
                        return;
                    }
                    let is_recording = app
                        .state::<AppState>()
                        .recorder
                        .lock()
                        .map(|value| value.is_some())
                        .unwrap_or(false);
                    if is_recording {
                        let _ = end_recording(app).await;
                    } else {
                        let _ = begin_recording(app).await;
                    }
                } else if event.state == ShortcutState::Pressed {
                    let _ = begin_recording(app).await;
                } else {
                    let _ = end_recording(app).await;
                }
            });
        })
        .build();
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _, _| {
            let _ = show_dashboard(app.clone());
        }))
        .plugin(
            tauri_plugin_log::Builder::new()
                .level(log::LevelFilter::Info)
                .build(),
        )
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(
            tauri_plugin_autostart::Builder::new()
                .args(["--hidden"])
                .build(),
        )
        .plugin(shortcut_plugin)
        .setup(|app| {
            let data_dir = app.path().app_data_dir()?;
            let models_dir = data_dir.join("models");
            std::fs::create_dir_all(&models_dir)?;
            let store = Store::load(&data_dir).map_err(std::io::Error::other)?;
            app.manage(AppState {
                store: Mutex::new(store),
                recorder: Mutex::new(None),
                engine: Mutex::new(None),
                models_dir,
            });
            if let Ok(store) = app.state::<AppState>().store.lock() {
                refresh_shortcuts(app.handle(), &AppConfig::default(), &store.data.config);
            }
            #[cfg(target_os = "macos")]
            macos_fn::install(app.handle().clone());
            let startup_config = app
                .state::<AppState>()
                .store
                .lock()
                .ok()
                .map(|store| store.data.config.clone())
                .unwrap_or_default();
            if let Some(widget) = app.get_webview_window("widget") {
                position_widget(&widget);
                if !startup_config.show_widget || startup_config.auto_hide_widget {
                    let _ = widget.hide();
                }
            }
            if std::env::args().any(|arg| arg == "--hidden" || arg == "--minimized") {
                if let Some(dashboard) = app.get_webview_window("dashboard") {
                    let _ = dashboard.hide();
                }
            }
            use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
            use tauri::tray::TrayIconBuilder;
            let open = MenuItem::with_id(app, "open", "Open Bee", true, None::<&str>)?;
            let record = MenuItem::with_id(app, "record", "Start Recording", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "Quit Bee", true, None::<&str>)?;
            let sep = PredefinedMenuItem::separator(app)?;
            let menu = Menu::with_items(app, &[&open, &record, &sep, &quit])?;
            let mut tray = TrayIconBuilder::new()
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "open" => {
                        let _ = show_dashboard(app.clone());
                    }
                    "record" => {
                        let app = app.clone();
                        tauri::async_runtime::spawn(async move {
                            let is_recording = app
                                .state::<AppState>()
                                .recorder
                                .lock()
                                .map(|x| x.is_some())
                                .unwrap_or(false);
                            if is_recording {
                                let _ = end_recording(app).await;
                            } else {
                                let _ = begin_recording(app).await;
                            }
                        });
                    }
                    "quit" => {
                        app.exit(0);
                    }
                    _ => {}
                });
            if let Some(icon) = app.default_window_icon() {
                tray = tray.icon(icon.clone())
            }
            let _tray = tray.build(app)?;
            Ok(())
        })
        .on_window_event(|window, event| {
            if window.label() == "dashboard" {
                if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                    api.prevent_close();
                    let app = window.app_handle();
                    let close_notice_seen = app
                        .state::<AppState>()
                        .store
                        .lock()
                        .ok()
                        .is_some_and(|store| store.data.config.close_notice_seen);
                    if close_notice_seen {
                        let _ = window.hide();
                    } else {
                        let _ = app.emit("close-to-tray-requested", ());
                    }
                    if let Some(widget) = app.get_webview_window("widget") {
                        let config = app
                            .state::<AppState>()
                            .store
                            .lock()
                            .ok()
                            .map(|s| s.data.config.clone());
                        if config
                            .as_ref()
                            .is_some_and(|c| c.show_widget && !c.auto_hide_widget)
                        {
                            let _ = widget.show();
                        }
                    }
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            get_snapshot,
            update_config,
            start_recording,
            stop_recording,
            cancel_recording,
            select_model,
            download_model,
            delete_model,
            delete_history,
            clear_history,
            export_history,
            upsert_dictionary,
            delete_dictionary,
            copy_text,
            polish_transcription,
            set_groq_api_key,
            check_permissions,
            request_permissions,
            check_for_updates,
            show_dashboard,
            acknowledge_close_notice,
            quit_app
        ])
        .run(tauri::generate_context!())
        .expect("error while running Bee")
}
