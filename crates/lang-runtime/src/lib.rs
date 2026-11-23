pub mod generated_bridge;

use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::{Mutex, OnceLock};
use std::thread;

enum TweakKind {
    // Box keeps the RAM address stable for JIT-compiled code
    Int(Box<AtomicI64>),
    Str(Mutex<String>),
    Vec(Mutex<Vec<i64>>),
}

struct TweakEntry {
    name: String,
    kind: TweakKind,
}

static TWEAKS: OnceLock<Mutex<Vec<TweakEntry>>> = OnceLock::new();

fn registry() -> &'static Mutex<Vec<TweakEntry>> {
    TWEAKS.get_or_init(|| Mutex::new(Vec::new()))
}

/// Registers an integer `tweak` variable and returns its stable 8-byte RAM address for Cranelift.
pub fn register_tweak(name: &str, initial: i64) -> *const i64 {
    let mut list = registry().lock().unwrap();
    if let Some(existing) = list.iter().find(|e| e.name == name) {
        if let TweakKind::Int(boxed) = &existing.kind {
            return boxed.as_ptr();
        }
    }
    let boxed = Box::new(AtomicI64::new(initial));
    let ptr = boxed.as_ptr();
    list.push(TweakEntry {
        name: name.to_string(),
        kind: TweakKind::Int(boxed),
    });
    ptr
}

/// Registers a string `tweak` variable and returns its registry index.
pub fn register_str_tweak(name: &str, initial: &str) -> usize {
    let mut list = registry().lock().unwrap();
    if let Some(idx) = list.iter().position(|e| e.name == name) {
        return idx;
    }
    let idx = list.len();
    list.push(TweakEntry {
        name: name.to_string(),
        kind: TweakKind::Str(Mutex::new(initial.to_string())),
    });
    idx
}

/// Registers a `Vec<i64>` `tweak` variable and returns its registry index.
pub fn register_vec_tweak(name: &str, initial: &[i64]) -> usize {
    let mut list = registry().lock().unwrap();
    if let Some(idx) = list.iter().position(|e| e.name == name) {
        return idx;
    }
    let idx = list.len();
    list.push(TweakEntry {
        name: name.to_string(),
        kind: TweakKind::Vec(Mutex::new(initial.to_vec())),
    });
    idx
}

/// Clones the current value of a `Vec` `tweak` into a new heap `Vec<i64>`.
#[unsafe(no_mangle)]
pub extern "C" fn striv_tweak_get_vec(idx: usize) -> *mut Vec<i64> {
    let list = registry().lock().unwrap();
    let v = match list.get(idx) {
        Some(TweakEntry {
            kind: TweakKind::Vec(m),
            ..
        }) => m.lock().unwrap().clone(),
        _ => Vec::new(),
    };
    Box::into_raw(Box::new(v))
}

/// Clones the current value of a string `tweak` into a new heap `String`.
#[unsafe(no_mangle)]
pub extern "C" fn striv_tweak_get_str(idx: usize) -> *mut String {
    let list = registry().lock().unwrap();
    let s = match list.get(idx) {
        Some(TweakEntry {
            kind: TweakKind::Str(m),
            ..
        }) => m.lock().unwrap().clone(),
        _ => String::new(),
    };
    Box::into_raw(Box::new(s))
}

/// Starts the live tweak dashboard in the background on http://localhost:4040.
pub fn start_tweak_server() {
    thread::spawn(|| {
        let Ok(server) = tiny_http::Server::http("127.0.0.1:4040") else {
            return;
        };
        println!("[Striv] Live-Tweak Dashboard active at http://localhost:4040");

        for request in server.incoming_requests() {
            let url = request.url().to_string();

            // 1. Receive update from browser: /set?name=dmg&val=50
            if let Some(query) = url.strip_prefix("/set?") {
                let mut target = "";
                let mut raw_val = None;
                for pair in query.split('&') {
                    if let Some((k, v)) = pair.split_once('=') {
                        if k == "name" {
                            target = v;
                        }
                        if k == "val" {
                            raw_val = Some(v);
                        }
                    }
                }
                if let Some(val_str) = raw_val {
                    let list = registry().lock().unwrap();
                    if let Some(entry) = list.iter().find(|e| e.name == target) {
                        match &entry.kind {
                            TweakKind::Int(atom) => {
                                if let Ok(v) = val_str.parse::<i64>() {
                                    atom.store(v, Ordering::Relaxed);
                                }
                            }
                            TweakKind::Str(m) => {
                                let decoded = val_str.replace("%20", " ").replace('+', " ");
                                *m.lock().unwrap() = decoded;
                            }
                            TweakKind::Vec(m) => {
                                let decoded = val_str.replace("%2C", ",").replace("%20", " ");
                                let parsed: Vec<i64> = decoded
                                    .split(',')
                                    .filter_map(|s| s.trim().parse::<i64>().ok())
                                    .collect();
                                *m.lock().unwrap() = parsed;
                            }
                        }
                    }
                }
                let _ = request.respond(tiny_http::Response::from_string("ok"));
                continue;
            }

            // 2. Render HTML page with live sliders & text inputs
            let list = registry().lock().unwrap();
            let mut rows = String::new();
            for entry in list.iter() {
                match &entry.kind {
                    TweakKind::Int(atom) => {
                        let v = atom.load(Ordering::Relaxed);
                        let min = (v - 100).min(0);
                        let max = (v + 200).max(100);
                        rows.push_str(&format!(
                            r#"<div class="card">
                                <div class="row">
                                    <label for="num_{name}"><b>{name}</b></label>
                                    <input id="num_{name}" type="number" value="{v}"
                                        oninput="document.getElementById('rng_{name}').value=this.value; fetch('/set?name={name}&val='+this.value)">
                                </div>
                                <input id="rng_{name}" type="range" min="{min}" max="{max}" value="{v}"
                                    oninput="document.getElementById('num_{name}').value=this.value; fetch('/set?name={name}&val='+this.value)">
                            </div>"#,
                            name = entry.name,
                            v = v,
                            min = min,
                            max = max
                        ));
                    }
                    TweakKind::Str(m) => {
                        let s = m.lock().unwrap().clone();
                        rows.push_str(&format!(
                            r#"<div class="card">
                                <div class="row">
                                    <label for="txt_{name}"><b>{name}</b></label>
                                    <input id="txt_{name}" type="text" value="{s}"
                                        oninput="fetch('/set?name={name}&val='+encodeURIComponent(this.value))">
                                </div>
                            </div>"#,
                            name = entry.name,
                            s = s
                        ));
                    }
                    TweakKind::Vec(m) => {
                        let v = m.lock().unwrap().clone();
                        let s: Vec<String> = v.iter().map(|x| x.to_string()).collect();
                        let joined = s.join(", ");
                        rows.push_str(&format!(
                            r#"<div class="card">
                                <div class="row">
                                    <label for="vec_{name}"><b>{name}</b></label>
                                    <input id="vec_{name}" type="text" value="{joined}"
                                        oninput="fetch('/set?name={name}&val='+encodeURIComponent(this.value))">
                                </div>
                            </div>"#,
                            name = entry.name,
                            joined = joined
                        ));
                    }
                }
            }
            if rows.is_empty() {
                rows.push_str(
                    "<p>No <code>tweak</code> variables found in the current program.</p>",
                );
            }

            let html = format!(
                r#"<!DOCTYPE html><html><head><meta charset="utf-8"><title>Striv Tweaks</title>
                <style>
                    body {{ font-family: system-ui, sans-serif; background: #14161a; color: #f0f0f0; max-width: 500px; margin: 40px auto; padding: 0 20px; }}
                    h2 {{ color: #61afef; }}
                    .card {{ background: #1e222a; padding: 16px; border-radius: 8px; margin-bottom: 12px; }}
                    .row {{ display: flex; justify-content: space-between; align-items: center; }}
                    input[type=number], input[type=text] {{ width: 120px; background: #14161a; color: #f0f0f0; border: 1px solid #3a3f4b; border-radius: 4px; padding: 4px 8px; }}
                    input[type=range] {{ width: 100%; margin-top: 10px; accent-color: #61afef; }}
                </style></head>
                <body><h2>Striv Live-Tweaks</h2>{rows}</body></html>"#
            );
            let header = tiny_http::Header::from_bytes(
                &b"Content-Type"[..],
                &b"text/html; charset=utf-8"[..],
            )
            .unwrap();
            let _ = request.respond(tiny_http::Response::from_string(html).with_header(header));
        }
    });
}

/// Creates a new heap-allocated `String` from raw UTF-8 bytes.
#[unsafe(no_mangle)]
pub extern "C" fn striv_str_new(ptr: *const u8, len: usize) -> *mut String {
    let slice = if ptr.is_null() || len == 0 {
        &[]
    } else {
        unsafe { std::slice::from_raw_parts(ptr, len) }
    };
    let s = std::str::from_utf8(slice).unwrap_or("").to_string();
    Box::into_raw(Box::new(s))
}

/// Appends an integer to an existing heap `String` (for `"HP: {hero.hp}"`).
#[unsafe(no_mangle)]
pub extern "C" fn striv_str_push_int(s_ptr: *mut String, val: i64) -> *mut String {
    let s = unsafe { &mut *s_ptr };
    s.push_str(&val.to_string());
    s_ptr
}

/// Appends another heap `String` (for `"Hello {name}"`).
#[unsafe(no_mangle)]
pub extern "C" fn striv_str_push_str(s_ptr: *mut String, other_ptr: *const String) -> *mut String {
    let s = unsafe { &mut *s_ptr };
    let other = unsafe { &*other_ptr };
    s.push_str(other);
    s_ptr
}

/// Prints a heap `String` to stdout.
#[unsafe(no_mangle)]
pub extern "C" fn striv_str_print(s_ptr: *const String) {
    let s = unsafe { &*s_ptr };
    println!(">> {}", s);
}

/// Frees a heap `String` at scope exit.
#[unsafe(no_mangle)]
pub extern "C" fn striv_str_free(s_ptr: *mut String) {
    if !s_ptr.is_null() {
        unsafe {
            drop(Box::from_raw(s_ptr));
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn striv_vec_new() -> *mut Vec<i64> {
    Box::into_raw(Box::new(Vec::new()))
}

#[unsafe(no_mangle)]
pub extern "C" fn striv_vec_push(v_ptr: *mut Vec<i64>, val: i64) {
    unsafe {
        (*v_ptr).push(val);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn striv_vec_get(v_ptr: *const Vec<i64>, idx: i64) -> i64 {
    let v = unsafe { &*v_ptr };
    v[idx as usize]
}

#[unsafe(no_mangle)]
pub extern "C" fn striv_vec_set(v_ptr: *mut Vec<i64>, idx: i64, val: i64) {
    let v = unsafe { &mut *v_ptr };
    v[idx as usize] = val;
}

#[unsafe(no_mangle)]
pub extern "C" fn striv_vec_len(v_ptr: *const Vec<i64>) -> i64 {
    unsafe { (*v_ptr).len() as i64 }
}

#[unsafe(no_mangle)]
pub extern "C" fn striv_vec_free(v_ptr: *mut Vec<i64>) {
    if !v_ptr.is_null() {
        unsafe {
            drop(Box::from_raw(v_ptr));
        }
    }
}

use minifb::{Key, Window, WindowOptions};
use slotmap::{Key as SlotKey, KeyData, SlotMap, new_key_type};
use std::cell::RefCell;

new_key_type! { pub struct WinHandle; }

pub struct HostWindow {
    pub win: Window,
    pub buffer: Vec<u32>,
    pub width: usize,
    pub height: usize,
}

thread_local! {
    static WINDOWS: RefCell<SlotMap<WinHandle, HostWindow>> = RefCell::new(SlotMap::with_key());
}

/// Creates a host window and returns its 8-byte SlotMap handle (index + generation).
pub fn create_host_window(title: &str, width: usize, height: usize) -> i64 {
    let win =
        Window::new(title, width, height, WindowOptions::default()).expect("Failed to open window");
    let host_win = HostWindow {
        win,
        buffer: vec![0; width * height],
        width,
        height,
    };
    WINDOWS.with(|w| w.borrow_mut().insert(host_win).data().as_ffi() as i64)
}

/// Updates the window at ~60 FPS; returns `false` when the user presses ESC or closes it.
pub fn present_host_window(handle_bits: i64) -> bool {
    let handle = WinHandle::from(KeyData::from_ffi(handle_bits as u64));
    WINDOWS.with(|w| {
        let mut map = w.borrow_mut();
        let Some(hw) = map.get_mut(handle) else {
            return false;
        };
        if !hw.win.is_open() || hw.win.is_key_down(Key::Escape) {
            return false;
        }
        hw.win
            .update_with_buffer(&hw.buffer, hw.width, hw.height)
            .is_ok()
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn striv_clear(handle_bits: i64, color: i64) -> i64 {
    let handle = WinHandle::from(KeyData::from_ffi(handle_bits as u64));
    WINDOWS.with(|w| {
        if let Some(hw) = w.borrow_mut().get_mut(handle) {
            hw.buffer.fill(color as u32);
        }
    });
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn striv_rect(
    handle_bits: i64,
    x: i64,
    y: i64,
    w_rect: i64,
    h_rect: i64,
    color: i64,
) -> i64 {
    let handle = WinHandle::from(KeyData::from_ffi(handle_bits as u64));
    WINDOWS.with(|w| {
        let mut map = w.borrow_mut();
        let Some(hw) = map.get_mut(handle) else {
            return;
        };
        let x0 = x.max(0) as usize;
        let y0 = y.max(0) as usize;
        let x1 = (x + w_rect).max(0).min(hw.width as i64) as usize;
        let y1 = (y + h_rect).max(0).min(hw.height as i64) as usize;

        for py in y0..y1 {
            let row = py * hw.width;
            for px in x0..x1 {
                hw.buffer[row + px] = color as u32;
            }
        }
    });
    0
}

/// Keyboard query: 1 = Left, 2 = Right, 3 = Up, 4 = Down, 5 = Space
#[unsafe(no_mangle)]
pub extern "C" fn striv_key_down(handle_bits: i64, key_code: i64) -> i64 {
    let handle = WinHandle::from(KeyData::from_ffi(handle_bits as u64));
    let key = match key_code {
        1 => Key::Left,
        2 => Key::Right,
        3 => Key::Up,
        4 => Key::Down,
        5 => Key::Space,
        6 => Key::A,
        7 => Key::D,
        _ => return 0,
    };
    WINDOWS.with(|w| {
        w.borrow()
            .get(handle)
            .map(|hw| if hw.win.is_key_down(key) { 1 } else { 0 })
            .unwrap_or(0)
    })
}
