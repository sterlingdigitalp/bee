use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{SampleFormat, Stream};
use serde::Serialize;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Emitter};

pub struct Recorder {
    pub stream: Stream,
    pub samples: Arc<Mutex<Vec<f32>>>,
    pub sample_rate: u32,
    pub channels: u16,
    pub started: std::time::Instant,
}
unsafe impl Send for Recorder {}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioDevice {
    pub name: String,
    pub is_default: bool,
}
pub fn list_devices() -> Vec<AudioDevice> {
    let host = cpal::default_host();
    let default = host.default_input_device().map(|d| d.to_string());
    host.input_devices()
        .map(|items| {
            items
                .map(|d| d.to_string())
                .map(|name| AudioDevice {
                    is_default: default.as_deref() == Some(&name),
                    name,
                })
                .collect()
        })
        .unwrap_or_default()
}

pub fn start(
    app: AppHandle,
    preferred: Option<&str>,
    fallback: Option<&str>,
    gain: f32,
) -> Result<Recorder, String> {
    let host = cpal::default_host();
    let devices: Vec<_> = host.input_devices().map_err(|e| e.to_string())?.collect();
    let mut candidates = Vec::new();
    for device in [preferred, fallback]
        .into_iter()
        .flatten()
        .filter_map(|name| devices.iter().find(|d| d.to_string() == name).cloned())
        .chain(host.default_input_device())
        .chain(devices.iter().cloned())
    {
        if !candidates
            .iter()
            .any(|existing: &cpal::Device| existing.to_string() == device.to_string())
        {
            candidates.push(device);
        }
    }
    if candidates.is_empty() {
        return Err("No microphone detected".into());
    }
    let mut errors = Vec::new();
    for device in candidates {
        let name = device.to_string();
        match start_device(app.clone(), device, gain) {
            Ok(recorder) => return Ok(recorder),
            Err(error) => errors.push(format!("{name}: {error}")),
        }
    }
    Err(format!(
        "No available microphone could be opened ({})",
        errors.join("; ")
    ))
}

fn start_device(app: AppHandle, device: cpal::Device, gain: f32) -> Result<Recorder, String> {
    let device_name = device.to_string();
    let supported = device.default_input_config().map_err(|e| e.to_string())?;
    let sample_rate = supported.sample_rate();
    let channels = supported.channels();
    let config = supported.config();
    let samples = Arc::new(Mutex::new(Vec::<f32>::new()));
    let tick = Arc::new(AtomicUsize::new(0));
    macro_rules! build {
        ($ty:ty,$convert:expr) => {{
            let app = app.clone();
            let callback_samples = samples.clone();
            let tick_cb = tick.clone();
            let err_app = app.clone();
            let cfg = config.clone();
            device
                .build_input_stream(
                    cfg,
                    move |data: &[$ty], _| {
                        let mut peak = 0f32;
                        if let Ok(mut out) = callback_samples.lock() {
                            for v in data {
                                let s: $ty = *v;
                                let f: f32 = $convert(s) * gain;
                                peak = peak.max(f.abs());
                                out.push(f.clamp(-1.0, 1.0));
                            }
                        }
                        if tick_cb.fetch_add(1, Ordering::Relaxed) % 4 == 0 {
                            let levels: Vec<f32> = (0..7)
                                .map(|i| (peak * (0.55 + i as f32 * 0.08)).min(1.0))
                                .collect();
                            let _ = app.emit("audio-levels", levels);
                        }
                    },
                    move |err| {
                        let _ = err_app.emit("recording-state", "error");
                        let _ = err_app.emit("recording-error", err.to_string());
                    },
                    None,
                )
                .map_err(|e| e.to_string())?
        }};
    }
    let stream = match supported.sample_format() {
        SampleFormat::F32 => build!(f32, |x: f32| x),
        SampleFormat::I16 => build!(i16, |x: i16| x as f32 / i16::MAX as f32),
        SampleFormat::U16 => build!(u16, |x: u16| (x as f32 / u16::MAX as f32) * 2.0 - 1.0),
        other => return Err(format!("Unsupported microphone format: {other:?}")),
    };
    stream.play().map_err(|e| e.to_string())?;
    let _ = app.emit("recording-device", &device_name);
    Ok(Recorder {
        stream,
        samples,
        sample_rate,
        channels,
        started: std::time::Instant::now(),
    })
}

pub fn finish(recorder: Recorder) -> (Vec<f32>, f32) {
    let duration = recorder.started.elapsed().as_secs_f32();
    drop(recorder.stream);
    let raw = recorder
        .samples
        .lock()
        .map(|x| x.clone())
        .unwrap_or_default();
    let mono = if recorder.channels <= 1 {
        raw
    } else {
        raw.chunks(recorder.channels as usize)
            .map(|f| f.iter().sum::<f32>() / f.len() as f32)
            .collect()
    };
    (resample(&mono, recorder.sample_rate, 16000), duration)
}

pub fn play_chime(rising: bool) {
    std::thread::spawn(move || {
        let host = cpal::default_host();
        let Some(device) = host.default_output_device() else {
            return;
        };
        let Ok(supported) = device.default_output_config() else {
            return;
        };
        let sample_rate = supported.sample_rate() as f32;
        let channels = supported.channels() as usize;
        let config = supported.config();
        macro_rules! tone_stream {
            ($ty:ty, $convert:expr) => {{
                let mut frame = 0u64;
                device.build_output_stream(
                    config.clone(),
                    move |data: &mut [$ty], _| {
                        for values in data.chunks_mut(channels) {
                            let progress = (frame as f32 / (sample_rate * 0.14)).min(1.0);
                            let frequency = if rising {
                                480.0 + 260.0 * progress
                            } else {
                                740.0 - 260.0 * progress
                            };
                            let envelope = (1.0 - progress).powi(2);
                            let value = (std::f32::consts::TAU * frequency * frame as f32
                                / sample_rate)
                                .sin()
                                * 0.075
                                * envelope;
                            let sample: $ty = $convert(value);
                            for channel in values {
                                *channel = sample;
                            }
                            frame += 1;
                        }
                    },
                    |_| {},
                    None,
                )
            }};
        }
        let stream = match supported.sample_format() {
            SampleFormat::F32 => tone_stream!(f32, |v: f32| v),
            SampleFormat::I16 => tone_stream!(i16, |v: f32| (v * i16::MAX as f32) as i16),
            SampleFormat::U16 => {
                tone_stream!(u16, |v: f32| ((v * 0.5 + 0.5) * u16::MAX as f32) as u16)
            }
            _ => return,
        };
        if let Ok(stream) = stream {
            if stream.play().is_ok() {
                std::thread::sleep(std::time::Duration::from_millis(160));
            }
        }
    });
}
fn resample(input: &[f32], from: u32, to: u32) -> Vec<f32> {
    if from == to {
        return input.to_vec();
    }
    let ratio = from as f64 / to as f64;
    let len = (input.len() as f64 / ratio) as usize;
    (0..len)
        .map(|i| {
            let pos = i as f64 * ratio;
            let a = pos.floor() as usize;
            let b = (a + 1).min(input.len().saturating_sub(1));
            let t = (pos - a as f64) as f32;
            input.get(a).copied().unwrap_or(0.0) * (1.0 - t)
                + input.get(b).copied().unwrap_or(0.0) * t
        })
        .collect()
}
