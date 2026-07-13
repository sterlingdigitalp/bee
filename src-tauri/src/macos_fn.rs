use core_foundation_sys::{
    base::{kCFAllocatorDefault, CFRelease, CFTypeRef},
    mach_port::{CFMachPortCreateRunLoopSource, CFMachPortRef},
    runloop::{kCFRunLoopCommonModes, CFRunLoopAddSource, CFRunLoopGetCurrent, CFRunLoopRun},
};
use std::{ffi::c_void, sync::mpsc, thread};
use tauri::{AppHandle, Emitter, Manager};

type CGEventRef = *mut c_void;
type CGEventTapProxy = *mut c_void;
type CGEventType = u32;
type CGEventFlags = u64;
type CGEventTapCallback =
    unsafe extern "C" fn(CGEventTapProxy, CGEventType, CGEventRef, *mut c_void) -> CGEventRef;

const FLAGS_CHANGED: CGEventType = 12;
const KEY_DOWN: CGEventType = 10;
const KEYBOARD_EVENT_KEYCODE: u32 = 9;
const ESCAPE_KEYCODE: i64 = 53;
const FLAG_SECONDARY_FN: CGEventFlags = 0x0080_0000;
const FLAG_SHIFT: CGEventFlags = 0x0002_0000;
const FLAG_CONTROL: CGEventFlags = 0x0004_0000;
const FLAG_OPTION: CGEventFlags = 0x0008_0000;
const FLAG_COMMAND: CGEventFlags = 0x0010_0000;
const HID_EVENT_TAP: u32 = 0;
const HEAD_INSERT_EVENT_TAP: u32 = 0;
const LISTEN_ONLY: u32 = 1;

#[link(name = "ApplicationServices", kind = "framework")]
extern "C" {
    fn CGEventTapCreate(
        tap: u32,
        place: u32,
        options: u32,
        events_of_interest: u64,
        callback: CGEventTapCallback,
        user_info: *mut c_void,
    ) -> CFMachPortRef;
    fn CGEventGetFlags(event: CGEventRef) -> CGEventFlags;
    fn CGEventGetIntegerValueField(event: CGEventRef, field: u32) -> i64;
    fn CGEventTapEnable(tap: CFMachPortRef, enable: bool);
    fn CGPreflightListenEventAccess() -> bool;
    fn CGRequestListenEventAccess() -> bool;
    fn AXIsProcessTrusted() -> bool;
}

pub fn input_monitoring_allowed() -> bool {
    unsafe { CGPreflightListenEventAccess() }
}
pub fn request_input_monitoring() -> bool {
    unsafe { CGRequestListenEventAccess() }
}
pub fn accessibility_allowed() -> bool {
    unsafe { AXIsProcessTrusted() }
}

struct Context {
    tx: mpsc::Sender<InputEvent>,
}

enum InputEvent {
    Flags(CGEventFlags),
    Cancel,
}

unsafe extern "C" fn callback(
    _proxy: CGEventTapProxy,
    event_type: CGEventType,
    event: CGEventRef,
    user_info: *mut c_void,
) -> CGEventRef {
    if event_type == FLAGS_CHANGED && !user_info.is_null() && !event.is_null() {
        let context = &mut *(user_info as *mut Context);
        let _ = context.tx.send(InputEvent::Flags(CGEventGetFlags(event)));
    } else if event_type == KEY_DOWN
        && !user_info.is_null()
        && !event.is_null()
        && CGEventGetIntegerValueField(event, KEYBOARD_EVENT_KEYCODE) == ESCAPE_KEYCODE
    {
        let context = &mut *(user_info as *mut Context);
        let _ = context.tx.send(InputEvent::Cancel);
    }
    event
}

fn modifier_mask(shortcut: &str) -> CGEventFlags {
    let lower = shortcut.to_lowercase();
    let mut remainder = lower.clone();
    for token in [
        "fn", "globe", "command", "cmd", "control", "ctrl", "option", "opt", "alt", "shift", "⌘",
        "⌃", "⌥", "⇧", "+", "/", "-", " ",
    ] {
        remainder = remainder.replace(token, "");
    }
    if !remainder.is_empty() {
        return 0;
    }
    let mut mask = 0;
    if lower.contains("fn") || lower.contains("globe") {
        mask |= FLAG_SECONDARY_FN;
    }
    if shortcut.contains('⌘') || lower.contains("command") || lower.contains("cmd") {
        mask |= FLAG_COMMAND;
    }
    if shortcut.contains('⌃') || lower.contains("control") || lower.contains("ctrl") {
        mask |= FLAG_CONTROL;
    }
    if shortcut.contains('⌥')
        || lower.contains("option")
        || lower.contains("opt")
        || lower.contains("alt")
    {
        mask |= FLAG_OPTION;
    }
    if shortcut.contains('⇧') || lower.contains("shift") {
        mask |= FLAG_SHIFT;
    }
    mask
}

pub fn install(app: AppHandle) {
    let (tx, rx) = mpsc::channel::<InputEvent>();
    let event_app = app.clone();
    thread::Builder::new()
        .name("bridgevoice-fn-dispatch".into())
        .spawn(move || {
            let mut shortcut_pressed = false;
            while let Ok(event) = rx.recv() {
                let app = event_app.clone();
                let pressed = match event {
                    InputEvent::Cancel => {
                        let state = app.state::<super::AppState>();
                        if let Ok(mut recorder) = state.recorder.lock() {
                            if let Some(recorder) = recorder.take() {
                                drop(recorder.stream);
                                let _ = app.emit("recording-state", "idle");
                            }
                        }
                        continue;
                    }
                    InputEvent::Flags(flags) => {
                        let key = app
                            .state::<super::AppState>()
                            .store
                            .lock()
                            .ok()
                            .map(|store| store.data.config.push_to_talk_key.clone())
                            .unwrap_or_default();
                        let mask = modifier_mask(&key);
                        let pressed = mask != 0 && flags & mask == mask;
                        if pressed == shortcut_pressed {
                            continue;
                        }
                        shortcut_pressed = pressed;
                        pressed
                    }
                };
                tauri::async_runtime::spawn(async move {
                    let enabled = app
                        .state::<super::AppState>()
                        .store
                        .lock()
                        .ok()
                        .is_some_and(|store| {
                            !store.data.config.shortcuts_paused
                                && modifier_mask(&store.data.config.push_to_talk_key) != 0
                        });
                    if !enabled {
                        return;
                    }
                    if pressed {
                        let _ = super::begin_recording(app).await;
                    } else {
                        let _ = super::end_recording(app).await;
                    }
                });
            }
        })
        .ok();

    thread::Builder::new()
        .name("bridgevoice-fn-monitor".into())
        .spawn(move || unsafe {
            let context = Box::new(Context { tx });
            let context_ptr = Box::into_raw(context) as *mut c_void;
            let tap = CGEventTapCreate(
                HID_EVENT_TAP,
                HEAD_INSERT_EVENT_TAP,
                LISTEN_ONLY,
                (1u64 << FLAGS_CHANGED) | (1u64 << KEY_DOWN),
                callback,
                context_ptr,
            );
            if tap.is_null() {
                let _ = Box::from_raw(context_ptr as *mut Context);
                let _ = app.emit(
                    "fn-monitor-unavailable",
                    "Grant Input Monitoring permission, then restart BridgeVoice.",
                );
                return;
            }
            CGEventTapEnable(tap, true);
            let source = CFMachPortCreateRunLoopSource(kCFAllocatorDefault, tap, 0);
            if source.is_null() {
                CFRelease(tap as CFTypeRef);
                return;
            }
            CFRunLoopAddSource(CFRunLoopGetCurrent(), source, kCFRunLoopCommonModes);
            CFRelease(source as CFTypeRef);
            CFRunLoopRun();
            CFRelease(tap as CFTypeRef);
            let _ = Box::from_raw(context_ptr as *mut Context);
        })
        .ok();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recognizes_modifier_only_shortcuts() {
        assert_eq!(modifier_mask("Fn / Globe"), FLAG_SECONDARY_FN);
        assert_eq!(modifier_mask("⌃ ⌥"), FLAG_CONTROL | FLAG_OPTION);
        assert_eq!(
            modifier_mask("Control + Option"),
            FLAG_CONTROL | FLAG_OPTION
        );
    }

    #[test]
    fn leaves_keyed_shortcuts_to_global_hotkey() {
        assert_eq!(modifier_mask("⌘ ⇧ R"), 0);
    }
}
