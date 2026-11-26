pub mod collections;
pub mod ecs;
pub mod generated_bridge;
pub mod json_io;
pub use collections::*;
pub use ecs::*;
pub use json_io::*;

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
fn url_decode(s: &str) -> String {
    let mut out = Vec::with_capacity(s.len());
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let (Some(h), Some(l)) = (
                (bytes[i + 1] as char).to_digit(16),
                (bytes[i + 2] as char).to_digit(16),
            ) {
                out.push(((h << 4) | l) as u8);
                i += 3;
                continue;
            }
        } else if bytes[i] == b'+' {
            out.push(b' ');
            i += 1;
            continue;
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn json_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            _ => out.push(c),
        }
    }
    out
}

fn apply_tweak_value(target: &str, val_str: &str) {
    let list = registry().lock().unwrap();
    if let Some(entry) = list.iter().find(|e| e.name == target) {
        match &entry.kind {
            TweakKind::Int(atom) => {
                if let Ok(v) = val_str.trim().parse::<i64>() {
                    atom.store(v, Ordering::Relaxed);
                }
            }
            TweakKind::Str(m) => {
                *m.lock().unwrap() = val_str.to_string();
            }
            TweakKind::Vec(m) => {
                let parsed: Vec<i64> = val_str
                    .split(',')
                    .filter_map(|s| s.trim().parse::<i64>().ok())
                    .collect();
                *m.lock().unwrap() = parsed;
            }
        }
    }
}

fn tweaks_as_json() -> String {
    let list = registry().lock().unwrap();
    let mut out = String::from("[");
    for (i, entry) in list.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        let group = entry
            .name
            .split_once('.')
            .or_else(|| entry.name.split_once('_'))
            .map(|(g, _)| g)
            .unwrap_or("General");
        match &entry.kind {
            TweakKind::Int(atom) => {
                let v = atom.load(Ordering::Relaxed);
                out.push_str(&format!(
                    r#"{{"name":"{}","group":"{}","type":"int","val":{}}}"#,
                    json_escape(&entry.name),
                    json_escape(group),
                    v
                ));
            }
            TweakKind::Str(m) => {
                let s = m.lock().unwrap().clone();
                out.push_str(&format!(
                    r#"{{"name":"{}","group":"{}","type":"str","val":"{}"}}"#,
                    json_escape(&entry.name),
                    json_escape(group),
                    json_escape(&s)
                ));
            }
            TweakKind::Vec(m) => {
                let v = m.lock().unwrap().clone();
                let joined: Vec<String> = v.iter().map(|x| x.to_string()).collect();
                out.push_str(&format!(
                    r#"{{"name":"{}","group":"{}","type":"vec","val":"{}"}}"#,
                    json_escape(&entry.name),
                    json_escape(group),
                    json_escape(&joined.join(", "))
                ));
            }
        }
    }
    out.push(']');
    out
}

const TWEAK_DASHBOARD_HTML: &str = r#"<!DOCTYPE html>
<html lang="de">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width,initial-scale=1">
<title>Striv Live-Tweaks</title>
<style>
  :root { --bg:#0f1117; --card:#181b23; --card2:#1f2430; --border:#2b3140; --fg:#e6edf3; --muted:#8b949e; --accent:#58a6ff; --green:#3fb950; --warn:#d29922; }
  * { box-sizing: border-box; }
  body { font-family: system-ui, -apple-system, sans-serif; background: var(--bg); color: var(--fg); max-width: 960px; margin: 0 auto; padding: 20px; }
  header { display:flex; flex-wrap:wrap; gap:12px; align-items:center; justify-content:space-between; margin-bottom:16px; }
  h2 { margin:0; font-size:20px; color:var(--accent); display:flex; align-items:center; gap:8px; }
  .badge { font-size:12px; background:var(--card2); border:1px solid var(--border); padding:2px 8px; border-radius:999px; color:var(--muted); }
  .toolbar { display:flex; flex-wrap:wrap; gap:8px; margin-bottom:16px; position:sticky; top:0; background:var(--bg); padding:10px 0; z-index:10; border-bottom:1px solid var(--border); }
  .search { flex:1; min-width:200px; background:var(--card); color:var(--fg); border:1px solid var(--border); border-radius:6px; padding:8px 12px; font-size:14px; outline:none; }
  .search:focus { border-color:var(--accent); }
  button, .pill { background:var(--card2); color:var(--fg); border:1px solid var(--border); border-radius:6px; padding:7px 12px; font-size:13px; cursor:pointer; }
  button:hover, .pill:hover { border-color:var(--accent); }
  .pill.active { background:var(--accent); color:#0b0e14; border-color:var(--accent); font-weight:600; }
  .group { background:var(--card); border:1px solid var(--border); border-radius:8px; margin-bottom:12px; overflow:hidden; }
  .group-hdr { display:flex; justify-content:space-between; align-items:center; padding:10px 14px; background:var(--card2); cursor:pointer; user-select:none; font-weight:600; }
  .group-hdr:hover { background:#252b3b; }
  .group-body { padding:10px 14px; display:grid; grid-template-columns:repeat(auto-fill, minmax(280px, 1fr)); gap:10px; }
  .group.collapsed .group-body { display:none; }
  .tweak { background:var(--bg); border:1px solid var(--border); border-radius:6px; padding:10px 12px; }
  .tweak-top { display:flex; justify-content:space-between; align-items:center; gap:8px; margin-bottom:6px; }
  .tweak-name { font-weight:600; font-size:13px; word-break:break-all; }
  .tweak-type { font-size:11px; color:var(--muted); background:var(--card2); padding:1px 6px; border-radius:4px; }
  .tweak-ctrl { display:flex; gap:6px; align-items:center; }
  input[type=number], input[type=text] { flex:1; min-width:0; background:var(--card); color:var(--fg); border:1px solid var(--border); border-radius:4px; padding:5px 8px; font-size:13px; }
  input[type=range] { width:100%; margin-top:8px; accent-color:var(--accent); }
  .btn-xs { padding:4px 7px; font-size:12px; }
  .status { font-size:12px; color:var(--green); min-height:16px; }
</style>
</head>
<body>
<header>
  <h2>Striv Live-Tweaks <span id="countBadge" class="badge">0</span></h2>
  <div style="display:flex;gap:8px;align-items:center;">
    <span id="status" class="status"></span>
    <button onclick="copyExport()">Exportieren</button>
    <button onclick="resetAll()">Alle Reset</button>
  </div>
</header>
<div class="toolbar">
  <input id="search" class="search" type="search" placeholder="Suchen... (Shortcut: /)" oninput="render()">
  <button class="pill active" onclick="setFilter('all', this)">Alle</button>
  <button class="pill" onclick="setFilter('int', this)">Int</button>
  <button class="pill" onclick="setFilter('str', this)">String</button>
  <button class="pill" onclick="setFilter('vec', this)">Vec</button>
  <button onclick="toggleAllGroups(false)">Alle auf</button>
  <button onclick="toggleAllGroups(true)">Alle zu</button>
  <button id="liveBtn" class="pill active" onclick="toggleLive()">Live: AN</button>
</div>
<div id="app"></div>
<script>
let tweaks = [];
let initialVals = {};
let filterType = 'all';
let collapsed = JSON.parse(localStorage.getItem('striv_collapsed') || '{}');
let livePoll = true;
let sendTimer = {};

function setStatus(msg) {
  const el = document.getElementById('status');
  el.textContent = msg;
  clearTimeout(el._t);
  el._t = setTimeout(() => el.textContent = '', 1800);
}

function setFilter(t, btn) {
  filterType = t;
  document.querySelectorAll('.toolbar .pill[onclick^="setFilter"]').forEach(b => b.classList.remove('active'));
  btn.classList.add('active');
  render();
}

function toggleGroup(g) {
  collapsed[g] = !collapsed[g];
  localStorage.setItem('striv_collapsed', JSON.stringify(collapsed));
  render();
}

function toggleAllGroups(state) {
  const groups = [...new Set(tweaks.map(t => t.group))];
  groups.forEach(g => collapsed[g] = state);
  localStorage.setItem('striv_collapsed', JSON.stringify(collapsed));
  render();
}

function toggleLive() {
  livePoll = !livePoll;
  const b = document.getElementById('liveBtn');
  b.textContent = livePoll ? 'Live: AN' : 'Live: AUS';
  b.classList.toggle('active', livePoll);
}

function sendVal(name, val, immediate) {
  const item = tweaks.find(t => t.name === name);
  if (item) item.val = item.type === 'int' ? (parseInt(val, 10) || 0) : val;
  clearTimeout(sendTimer[name]);
  const run = () => fetch('/set?name=' + encodeURIComponent(name) + '&val=' + encodeURIComponent(val));
  if (immediate) run(); else sendTimer[name] = setTimeout(run, 50);
}

function resetOne(name) {
  if (!(name in initialVals)) return;
  sendVal(name, initialVals[name], true);
  render();
  setStatus('Reset: ' + name);
}

function resetAll() {
  tweaks.forEach(t => { if (t.name in initialVals) sendVal(t.name, initialVals[t.name], true); });
  render();
  setStatus('Alle Werte zurückgesetzt');
}

function copyExport() {
  const lines = tweaks.map(t => {
    if (t.type === 'str') return `tweak ${t.name} = "${t.val}"`;
    if (t.type === 'vec') return `tweak ${t.name} = [${t.val}]`;
    return `tweak ${t.name} = ${t.val}`;
  }).join('\n');
  navigator.clipboard.writeText(lines).then(() => setStatus('Code kopiert!'));
}

function esc(s) {
  return String(s).replace(/&/g,'&amp;').replace(/</g,'&lt;').replace(/"/g,'&quot;');
}

function render() {
  const q = document.getElementById('search').value.trim().toLowerCase();
  const filtered = tweaks.filter(t => {
    if (filterType !== 'all' && t.type !== filterType) return false;
    if (q && !t.name.toLowerCase().includes(q) && !t.group.toLowerCase().includes(q)) return false;
    return true;
  });
  document.getElementById('countBadge').textContent = `${filtered.length} / ${tweaks.length}`;

  const byGroup = {};
  filtered.forEach(t => { (byGroup[t.group] = byGroup[t.group] || []).push(t); });

  const app = document.getElementById('app');
  if (filtered.length === 0) {
    app.innerHTML = '<p style="color:var(--muted)">Keine passenden Tweaks gefunden.</p>';
    return;
  }

  let html = '';
  for (const [grp, items] of Object.entries(byGroup)) {
    const isCol = !!collapsed[grp] && !q;
    html += `<div class="group ${isCol ? 'collapsed' : ''}">
      <div class="group-hdr" onclick="toggleGroup('${esc(grp)}')">
        <span>${isCol ? '▸' : '▾'} ${esc(grp)}</span>
        <span class="badge">${items.length}</span>
      </div>
      <div class="group-body">`;
    for (const t of items) {
      const id = 'id_' + btoa(unescape(encodeURIComponent(t.name))).replace(/=/g,'');
      if (t.type === 'int') {
        const v = Number(t.val) || 0;
        const min = Math.min(0, v - 100);
        const max = Math.max(100, v + 200);
        html += `<div class="tweak">
          <div class="tweak-top"><span class="tweak-name">${esc(t.name)}</span><span class="tweak-type">int</span></div>
          <div class="tweak-ctrl">
            <input id="${id}_n" type="number" value="${v}" oninput="document.getElementById('${id}_r').value=this.value;sendVal('${esc(t.name)}',this.value,true)">
            <button class="btn-xs" title="Reset" onclick="resetOne('${esc(t.name)}')">↺</button>
          </div>
          <input id="${id}_r" type="range" min="${min}" max="${max}" value="${v}" oninput="document.getElementById('${id}_n').value=this.value;sendVal('${esc(t.name)}',this.value,true)">
        </div>`;
      } else {
        html += `<div class="tweak">
          <div class="tweak-top"><span class="tweak-name">${esc(t.name)}</span><span class="tweak-type">${t.type}</span></div>
          <div class="tweak-ctrl">
            <input type="text" value="${esc(t.val)}" oninput="sendVal('${esc(t.name)}',this.value,false)">
            <button class="btn-xs" title="Reset" onclick="resetOne('${esc(t.name)}')">↺</button>
          </div>
        </div>`;
      }
    }
    html += `</div></div>`;
  }
  app.innerHTML = html;
}

async function poll() {
  if (!livePoll) return;
  const active = document.activeElement;
  if (active && (active.tagName === 'INPUT')) return;
  try {
    const res = await fetch('/api/tweaks');
    const fresh = await res.json();
    fresh.forEach(t => { if (!(t.name in initialVals)) initialVals[t.name] = t.val; });
    if (JSON.stringify(fresh) !== JSON.stringify(tweaks)) {
      tweaks = fresh;
      render();
    }
  } catch (_) {}
}

document.addEventListener('keydown', e => {
  if (e.key === '/' && document.activeElement.tagName !== 'INPUT') {
    e.preventDefault();
    document.getElementById('search').focus();
  }
});

poll();
setInterval(poll, 700);
</script>
</body>
</html>"#;

pub fn start_tweak_server() {
    thread::spawn(|| {
        let Ok(server) = tiny_http::Server::http("127.0.0.1:4040") else {
            return;
        };
        println!("[Striv] Live-Tweak Dashboard active at http://localhost:4040");

        for request in server.incoming_requests() {
            let url = request.url().to_string();

            if let Some(query) = url.strip_prefix("/set?") {
                let mut target = String::new();
                let mut raw_val = None;
                for pair in query.split('&') {
                    if let Some((k, v)) = pair.split_once('=') {
                        if k == "name" {
                            target = url_decode(v);
                        } else if k == "val" {
                            raw_val = Some(url_decode(v));
                        }
                    }
                }
                if let Some(val_str) = raw_val {
                    apply_tweak_value(&target, &val_str);
                }
                let _ = request.respond(tiny_http::Response::from_string("ok"));
                continue;
            }

            if url == "/api/tweaks" {
                let json = tweaks_as_json();
                let header = tiny_http::Header::from_bytes(
                    &b"Content-Type"[..],
                    &b"application/json; charset=utf-8"[..],
                )
                .unwrap();
                let _ = request.respond(tiny_http::Response::from_string(json).with_header(header));
                continue;
            }

            let header = tiny_http::Header::from_bytes(
                &b"Content-Type"[..],
                &b"text/html; charset=utf-8"[..],
            )
            .unwrap();
            let _ = request.respond(
                tiny_http::Response::from_string(TWEAK_DASHBOARD_HTML).with_header(header),
            );
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

/// Appends a float to an existing heap `String` (for `"pos.x = {pos.x}"`).
#[unsafe(no_mangle)]
pub extern "C" fn striv_str_push_float(s_ptr: *mut String, val: f64) -> *mut String {
    let s = unsafe { &mut *s_ptr };
    s.push_str(&format!("{}", val));
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

/// Ensures a heap `String` is null-terminated in-place and returns a `*const u8` C-string pointer.
#[unsafe(no_mangle)]
pub extern "C" fn striv_str_to_cstr(s_ptr: *mut String) -> *const u8 {
    if s_ptr.is_null() {
        return b"\0".as_ptr();
    }
    let s = unsafe { &mut *s_ptr };
    s.reserve(1);
    unsafe {
        let end = s.as_mut_ptr().add(s.len());
        *end = 0;
    }
    s.as_ptr()
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
pub extern "C" fn striv_window_new(title_ptr: *const String, width: i64, height: i64) -> i64 {
    let title = if title_ptr.is_null() {
        "Striv Window"
    } else {
        unsafe { (*title_ptr).as_str() }
    };
    create_host_window(title, width.max(1) as usize, height.max(1) as usize)
}

#[unsafe(no_mangle)]
pub extern "C" fn striv_window_is_open(handle_bits: i64) -> i64 {
    let handle = WinHandle::from(KeyData::from_ffi(handle_bits as u64));
    WINDOWS.with(|w| {
        let map = w.borrow();
        map.get(handle)
            .is_some_and(|hw| hw.win.is_open() && !hw.win.is_key_down(Key::Escape)) as i64
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn striv_window_clear(handle_bits: i64, rgb: i64) -> i64 {
    let handle = WinHandle::from(KeyData::from_ffi(handle_bits as u64));
    WINDOWS.with(|w| {
        if let Some(hw) = w.borrow_mut().get_mut(handle) {
            hw.buffer.fill(rgb as u32);
        }
    });
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn striv_window_draw_rect(
    handle_bits: i64,
    x: i64,
    y: i64,
    rw: i64,
    rh: i64,
    rgb: i64,
) -> i64 {
    let handle = WinHandle::from(KeyData::from_ffi(handle_bits as u64));
    WINDOWS.with(|w| {
        if let Some(hw) = w.borrow_mut().get_mut(handle) {
            let x0 = x.max(0) as usize;
            let y0 = y.max(0) as usize;
            let x1 = (x + rw).clamp(0, hw.width as i64) as usize;
            let y1 = (y + rh).clamp(0, hw.height as i64) as usize;
            let col = rgb as u32;
            for py in y0..y1 {
                let row = py * hw.width;
                for px in x0..x1 {
                    hw.buffer[row + px] = col;
                }
            }
        }
    });
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn striv_window_present(handle_bits: i64) -> i64 {
    present_host_window(handle_bits) as i64
}

use std::cell::Cell;

thread_local! {
    static LAST_ERROR: Cell<i64> = const { Cell::new(0) };
}

#[unsafe(no_mangle)]
pub extern "C" fn striv_set_err(code: i64) -> i64 {
    LAST_ERROR.with(|e| e.set(if code == 0 { 1 } else { code }));
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn striv_take_err() -> i64 {
    LAST_ERROR.with(|e| {
        let val = e.get();
        e.set(0);
        val
    })
}

thread_local! {
    static TUPLE_BUF: std::cell::UnsafeCell<[u64; 8]> = const { std::cell::UnsafeCell::new([0; 8]) };
}

#[unsafe(no_mangle)]
pub extern "C" fn striv_tuple_buf_ptr() -> *mut u64 {
    TUPLE_BUF.with(|b| b.get() as *mut u64)
}

thread_local! {
    static ASSERT_FAILED: Cell<bool> = const { Cell::new(false) };
}

#[unsafe(no_mangle)]
pub extern "C" fn striv_assert(cond: i64) -> i64 {
    if cond == 0 {
        ASSERT_FAILED.with(|f| f.set(true));
    }
    0
}

pub fn take_assert_failed() -> bool {
    ASSERT_FAILED.with(|f| f.replace(false))
}

pub fn block_on_async<F: std::future::Future>(f: F) -> F::Output {
    use std::pin::pin;
    use std::sync::Arc;
    use std::task::{Context, Poll, Wake};
    struct ThreadWaker(std::thread::Thread);
    impl Wake for ThreadWaker {
        fn wake(self: Arc<Self>) {
            self.0.unpark();
        }
    }
    let waker = Arc::new(ThreadWaker(std::thread::current())).into();
    let mut cx = Context::from_waker(&waker);
    let mut f = pin!(f);
    loop {
        match f.as_mut().poll(&mut cx) {
            Poll::Ready(val) => return val,
            Poll::Pending => std::thread::park(),
        }
    }
}

thread_local! {
    static SANDBOX_GAS: std::cell::Cell<i64> = const { std::cell::Cell::new(i64::MAX) };
    static SANDBOX_ABORTED: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

pub fn set_sandbox_gas(limit: i64) {
    SANDBOX_GAS.with(|g| g.set(limit));
    SANDBOX_ABORTED.with(|a| a.set(false));
}

pub fn take_sandbox_aborted() -> bool {
    SANDBOX_ABORTED.with(|a| a.replace(false))
}

#[unsafe(no_mangle)]
pub extern "C" fn striv_check_gas() -> i64 {
    SANDBOX_GAS.with(|g| {
        let rem = g.get() - 1;
        g.set(rem);
        if rem <= 0 {
            SANDBOX_ABORTED.with(|a| a.set(true));
            1
        } else {
            0
        }
    })
}

static NEXT_ANY_HANDLE: AtomicI64 = AtomicI64::new(1);
static ANY_STORE: std::sync::OnceLock<
    parking_lot::RwLock<std::collections::HashMap<i64, Box<dyn std::any::Any + Send + Sync>>>,
> = std::sync::OnceLock::new();

fn any_map()
-> &'static parking_lot::RwLock<std::collections::HashMap<i64, Box<dyn std::any::Any + Send + Sync>>>
{
    ANY_STORE.get_or_init(|| parking_lot::RwLock::new(std::collections::HashMap::new()))
}

pub fn box_any_handle<T: std::any::Any + Send + Sync>(val: T) -> i64 {
    let id = NEXT_ANY_HANDLE.fetch_add(1, Ordering::Relaxed);
    any_map().write().insert(id, Box::new(val));
    id
}

pub fn with_any_ref<T: std::any::Any, R>(id: i64, f: impl FnOnce(&T) -> R) -> R {
    if id == 0 {
        panic!("[Striv] Null-Handle Zugriff (id == 0) in with_any_ref!");
    }
    let guard = any_map().read();
    let obj = guard
        .get(&id)
        .expect("Ungültiges Rust-Handle")
        .downcast_ref::<T>()
        .expect("Falscher Handle-Typ in rust fn");
    f(obj)
}

pub fn with_any_mut<T: std::any::Any, R>(id: i64, f: impl FnOnce(&mut T) -> R) -> R {
    if id == 0 {
        panic!("[Striv] Null-Handle Zugriff (id == 0) in with_any_mut!");
    }
    let mut guard = any_map().write();
    let obj = guard
        .get_mut(&id)
        .expect("Ungültiges Rust-Handle")
        .downcast_mut::<T>()
        .expect("Falscher Handle-Typ in rust fn");
    f(obj)
}

#[unsafe(no_mangle)]
pub extern "C" fn striv_any_free(id: i64) {
    if id > 0 {
        any_map().write().remove(&id);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn striv_sin(x: f64) -> f64 {
    x.sin()
}

#[unsafe(no_mangle)]
pub extern "C" fn striv_cos(x: f64) -> f64 {
    x.cos()
}

#[unsafe(no_mangle)]
pub extern "C" fn striv_tan(x: f64) -> f64 {
    x.tan()
}

#[unsafe(no_mangle)]
pub extern "C" fn striv_atan2(y: f64, x: f64) -> f64 {
    y.atan2(x)
}

#[unsafe(no_mangle)]
pub extern "C" fn striv_sqrt(x: f64) -> f64 {
    x.sqrt()
}

#[unsafe(no_mangle)]
pub extern "C" fn striv_abs(x: f64) -> f64 {
    x.abs()
}

#[unsafe(no_mangle)]
pub extern "C" fn striv_acos(x: f64) -> f64 {
    x.acos()
}

#[unsafe(no_mangle)]
pub extern "C" fn striv_asin(x: f64) -> f64 {
    x.asin()
}

#[unsafe(no_mangle)]
pub extern "C" fn striv_atan(x: f64) -> f64 {
    x.atan()
}

#[unsafe(no_mangle)]
pub extern "C" fn striv_sinh(x: f64) -> f64 {
    x.sinh()
}

#[unsafe(no_mangle)]
pub extern "C" fn striv_cosh(x: f64) -> f64 {
    x.cosh()
}

#[unsafe(no_mangle)]
pub extern "C" fn striv_asinh(x: f64) -> f64 {
    x.asinh()
}

#[unsafe(no_mangle)]
pub extern "C" fn striv_atanh(x: f64) -> f64 {
    x.atanh()
}

#[unsafe(no_mangle)]
pub extern "C" fn striv_rem_euclid(a: f64, b: f64) -> f64 {
    a.rem_euclid(b)
}
