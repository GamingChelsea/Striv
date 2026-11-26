use crossbeam_channel::{Receiver, Select, Sender, unbounded};
use parking_lot::RawMutex;
use parking_lot::lock_api::RawMutex as _;
use rayon::prelude::*;
use std::collections::{BTreeMap, BinaryHeap, HashMap, HashSet, VecDeque};
use std::sync::atomic::{AtomicI64, Ordering};
use std::time::Duration;

// --- 1. DEQUE (VecDeque<i64>) ---
#[unsafe(no_mangle)]
pub extern "C" fn striv_deque_new() -> *mut VecDeque<i64> {
    Box::into_raw(Box::new(VecDeque::new()))
}
#[unsafe(no_mangle)]
pub extern "C" fn striv_deque_push_back(p: *mut VecDeque<i64>, v: i64) -> i64 {
    unsafe { (*p).push_back(v) };
    0
}
#[unsafe(no_mangle)]
pub extern "C" fn striv_deque_push_front(p: *mut VecDeque<i64>, v: i64) -> i64 {
    unsafe { (*p).push_front(v) };
    0
}
#[unsafe(no_mangle)]
pub extern "C" fn striv_deque_pop_front(p: *mut VecDeque<i64>) -> i64 {
    unsafe { (*p).pop_front().unwrap_or(0) }
}
#[unsafe(no_mangle)]
pub extern "C" fn striv_deque_pop_back(p: *mut VecDeque<i64>) -> i64 {
    unsafe { (*p).pop_back().unwrap_or(0) }
}
#[unsafe(no_mangle)]
pub extern "C" fn striv_deque_len(p: *const VecDeque<i64>) -> i64 {
    unsafe { (*p).len() as i64 }
}
#[unsafe(no_mangle)]
pub extern "C" fn striv_deque_free(p: *mut VecDeque<i64>) {
    if !p.is_null() {
        unsafe { drop(Box::from_raw(p)) };
    }
}

// --- 2. MAP (HashMap<i64, i64>) & STRMAP (HashMap<String, i64>) ---
#[unsafe(no_mangle)]
pub extern "C" fn striv_map_new() -> *mut HashMap<i64, i64> {
    Box::into_raw(Box::new(HashMap::new()))
}
#[unsafe(no_mangle)]
pub extern "C" fn striv_map_insert(p: *mut HashMap<i64, i64>, k: i64, v: i64) -> i64 {
    unsafe { (*p).insert(k, v).unwrap_or(0) }
}
#[unsafe(no_mangle)]
pub extern "C" fn striv_map_get(p: *const HashMap<i64, i64>, k: i64) -> i64 {
    unsafe { *(*p).get(&k).unwrap_or(&0) }
}
#[unsafe(no_mangle)]
pub extern "C" fn striv_map_contains(p: *const HashMap<i64, i64>, k: i64) -> i64 {
    unsafe { (*p).contains_key(&k) as i64 }
}
#[unsafe(no_mangle)]
pub extern "C" fn striv_map_remove(p: *mut HashMap<i64, i64>, k: i64) -> i64 {
    unsafe { (*p).remove(&k).unwrap_or(0) }
}
#[unsafe(no_mangle)]
pub extern "C" fn striv_map_len(p: *const HashMap<i64, i64>) -> i64 {
    unsafe { (*p).len() as i64 }
}
#[unsafe(no_mangle)]
pub extern "C" fn striv_map_free(p: *mut HashMap<i64, i64>) {
    if !p.is_null() {
        unsafe { drop(Box::from_raw(p)) };
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn striv_strmap_new() -> *mut HashMap<String, i64> {
    Box::into_raw(Box::new(HashMap::new()))
}
#[unsafe(no_mangle)]
pub extern "C" fn striv_strmap_insert(
    p: *mut HashMap<String, i64>,
    k: *const String,
    v: i64,
) -> i64 {
    unsafe { (*p).insert((*k).clone(), v).unwrap_or(0) }
}
#[unsafe(no_mangle)]
pub extern "C" fn striv_strmap_get(p: *const HashMap<String, i64>, k: *const String) -> i64 {
    unsafe { *(*p).get(&*k).unwrap_or(&0) }
}
#[unsafe(no_mangle)]
pub extern "C" fn striv_strmap_contains(p: *const HashMap<String, i64>, k: *const String) -> i64 {
    unsafe { (*p).contains_key(&*k) as i64 }
}
#[unsafe(no_mangle)]
pub extern "C" fn striv_strmap_free(p: *mut HashMap<String, i64>) {
    if !p.is_null() {
        unsafe { drop(Box::from_raw(p)) };
    }
}

// --- 3. BTREE (BTreeMap<i64, i64>) ---
#[unsafe(no_mangle)]
pub extern "C" fn striv_btree_new() -> *mut BTreeMap<i64, i64> {
    Box::into_raw(Box::new(BTreeMap::new()))
}
#[unsafe(no_mangle)]
pub extern "C" fn striv_btree_insert(p: *mut BTreeMap<i64, i64>, k: i64, v: i64) -> i64 {
    unsafe { (*p).insert(k, v).unwrap_or(0) }
}
#[unsafe(no_mangle)]
pub extern "C" fn striv_btree_get(p: *const BTreeMap<i64, i64>, k: i64) -> i64 {
    unsafe { *(*p).get(&k).unwrap_or(&0) }
}
#[unsafe(no_mangle)]
pub extern "C" fn striv_btree_first_key(p: *const BTreeMap<i64, i64>) -> i64 {
    unsafe { *(*p).keys().next().unwrap_or(&0) }
}
#[unsafe(no_mangle)]
pub extern "C" fn striv_btree_last_key(p: *const BTreeMap<i64, i64>) -> i64 {
    unsafe { *(*p).keys().next_back().unwrap_or(&0) }
}
#[unsafe(no_mangle)]
pub extern "C" fn striv_btree_free(p: *mut BTreeMap<i64, i64>) {
    if !p.is_null() {
        unsafe { drop(Box::from_raw(p)) };
    }
}

// --- 4. SET (HashSet<i64>) ---
#[unsafe(no_mangle)]
pub extern "C" fn striv_set_new() -> *mut HashSet<i64> {
    Box::into_raw(Box::new(HashSet::new()))
}
#[unsafe(no_mangle)]
pub extern "C" fn striv_set_insert(p: *mut HashSet<i64>, v: i64) -> i64 {
    unsafe { (*p).insert(v) as i64 }
}
#[unsafe(no_mangle)]
pub extern "C" fn striv_set_contains(p: *const HashSet<i64>, v: i64) -> i64 {
    unsafe { (*p).contains(&v) as i64 }
}
#[unsafe(no_mangle)]
pub extern "C" fn striv_set_remove(p: *mut HashSet<i64>, v: i64) -> i64 {
    unsafe { (*p).remove(&v) as i64 }
}
#[unsafe(no_mangle)]
pub extern "C" fn striv_set_len(p: *const HashSet<i64>) -> i64 {
    unsafe { (*p).len() as i64 }
}
#[unsafe(no_mangle)]
pub extern "C" fn striv_set_free(p: *mut HashSet<i64>) {
    if !p.is_null() {
        unsafe { drop(Box::from_raw(p)) };
    }
}

// --- 5. HEAP (BinaryHeap<i64>) ---
#[unsafe(no_mangle)]
pub extern "C" fn striv_heap_new() -> *mut BinaryHeap<i64> {
    Box::into_raw(Box::new(BinaryHeap::new()))
}
#[unsafe(no_mangle)]
pub extern "C" fn striv_heap_push(p: *mut BinaryHeap<i64>, v: i64) -> i64 {
    unsafe { (*p).push(v) };
    0
}
#[unsafe(no_mangle)]
pub extern "C" fn striv_heap_pop(p: *mut BinaryHeap<i64>) -> i64 {
    unsafe { (*p).pop().unwrap_or(0) }
}
#[unsafe(no_mangle)]
pub extern "C" fn striv_heap_peek(p: *const BinaryHeap<i64>) -> i64 {
    unsafe { *(*p).peek().unwrap_or(&0) }
}
#[unsafe(no_mangle)]
pub extern "C" fn striv_heap_len(p: *const BinaryHeap<i64>) -> i64 {
    unsafe { (*p).len() as i64 }
}
#[unsafe(no_mangle)]
pub extern "C" fn striv_heap_free(p: *mut BinaryHeap<i64>) {
    if !p.is_null() {
        unsafe { drop(Box::from_raw(p)) };
    }
}

// --- 6. GRID (2D Array) ---
pub struct Grid2D {
    pub w: usize,
    pub h: usize,
    pub cells: Vec<i64>,
}
#[unsafe(no_mangle)]
pub extern "C" fn striv_grid_new(w: i64, h: i64, fill: i64) -> *mut Grid2D {
    let (wu, hu) = (w.max(1) as usize, h.max(1) as usize);
    Box::into_raw(Box::new(Grid2D {
        w: wu,
        h: hu,
        cells: vec![fill; wu * hu],
    }))
}
#[unsafe(no_mangle)]
pub extern "C" fn striv_grid_get(p: *const Grid2D, x: i64, y: i64) -> i64 {
    let g = unsafe { &*p };
    if x < 0 || y < 0 || (x as usize) >= g.w || (y as usize) >= g.h {
        return 0;
    }
    g.cells[(y as usize) * g.w + (x as usize)]
}
#[unsafe(no_mangle)]
pub extern "C" fn striv_grid_set(p: *mut Grid2D, x: i64, y: i64, val: i64) -> i64 {
    let g = unsafe { &mut *p };
    if x >= 0 && y >= 0 && (x as usize) < g.w && (y as usize) < g.h {
        g.cells[(y as usize) * g.w + (x as usize)] = val;
    }
    0
}
#[unsafe(no_mangle)]
pub extern "C" fn striv_grid_in_bounds(p: *const Grid2D, x: i64, y: i64) -> i64 {
    let g = unsafe { &*p };
    (x >= 0 && y >= 0 && (x as usize) < g.w && (y as usize) < g.h) as i64
}
#[unsafe(no_mangle)]
pub extern "C" fn striv_grid_free(p: *mut Grid2D) {
    if !p.is_null() {
        unsafe { drop(Box::from_raw(p)) };
    }
}

// --- 7. BYTES (Vec<u8> + File I/O) ---
#[unsafe(no_mangle)]
pub extern "C" fn striv_bytes_new() -> *mut Vec<u8> {
    Box::into_raw(Box::new(Vec::new()))
}
#[unsafe(no_mangle)]
pub extern "C" fn striv_bytes_push(p: *mut Vec<u8>, b: i64) -> i64 {
    unsafe { (*p).push(b as u8) };
    0
}
#[unsafe(no_mangle)]
pub extern "C" fn striv_bytes_get(p: *const Vec<u8>, idx: i64) -> i64 {
    unsafe { (&(*p))[idx as usize] as i64 }
}
#[unsafe(no_mangle)]
pub extern "C" fn striv_bytes_len(p: *const Vec<u8>) -> i64 {
    unsafe { (*p).len() as i64 }
}
#[unsafe(no_mangle)]
pub extern "C" fn striv_bytes_read_file(path: *const String) -> *mut Vec<u8> {
    let data = unsafe { std::fs::read(&*path).unwrap_or_default() };
    Box::into_raw(Box::new(data))
}
#[unsafe(no_mangle)]
pub extern "C" fn striv_bytes_free(p: *mut Vec<u8>) {
    if !p.is_null() {
        unsafe { drop(Box::from_raw(p)) };
    }
}

// --- 8. RNG (Fast Xorshift64) ---
pub struct FastRng(u64);
#[unsafe(no_mangle)]
pub extern "C" fn striv_rng_new(seed: i64) -> *mut FastRng {
    Box::into_raw(Box::new(FastRng(if seed == 0 {
        0x9E3779B97F4A7C15
    } else {
        seed as u64
    })))
}
#[unsafe(no_mangle)]
pub extern "C" fn striv_rng_range(p: *mut FastRng, min: i64, max: i64) -> i64 {
    let s = unsafe { &mut (*p).0 };
    *s ^= *s << 13;
    *s ^= *s >> 7;
    *s ^= *s << 17;
    if max <= min {
        return min;
    }
    min + ((*s % ((max - min) as u64)) as i64)
}
#[unsafe(no_mangle)]
pub extern "C" fn striv_rng_free(p: *mut FastRng) {
    if !p.is_null() {
        unsafe { drop(Box::from_raw(p)) };
    }
}

// --- 9. BITSET (Compact bit flags) ---
pub struct BitSet(Vec<u64>);
#[unsafe(no_mangle)]
pub extern "C" fn striv_bitset_new(bits: i64) -> *mut BitSet {
    let words = ((bits.max(1) as usize) + 63) / 64;
    Box::into_raw(Box::new(BitSet(vec![0; words])))
}
#[unsafe(no_mangle)]
pub extern "C" fn striv_bitset_set(p: *mut BitSet, bit: i64) -> i64 {
    if bit >= 0 {
        let b = bit as usize;
        let w = b / 64;
        let v = unsafe { &mut (*p).0 };
        if w < v.len() {
            v[w] |= 1u64 << (b % 64);
        }
    }
    0
}
#[unsafe(no_mangle)]
pub extern "C" fn striv_bitset_get(p: *const BitSet, bit: i64) -> i64 {
    if bit < 0 {
        return 0;
    }
    let b = bit as usize;
    let w = b / 64;
    let v = unsafe { &(*p).0 };
    if w < v.len() {
        ((v[w] >> (b % 64)) & 1) as i64
    } else {
        0
    }
}
#[unsafe(no_mangle)]
pub extern "C" fn striv_bitset_clear(p: *mut BitSet, bit: i64) -> i64 {
    if bit >= 0 {
        let b = bit as usize;
        let w = b / 64;
        let v = unsafe { &mut (*p).0 };
        if w < v.len() {
            v[w] &= !(1u64 << (b % 64));
        }
    }
    0
}
#[unsafe(no_mangle)]
pub extern "C" fn striv_bitset_free(p: *mut BitSet) {
    if !p.is_null() {
        unsafe { drop(Box::from_raw(p)) };
    }
}

// --- 10. CHANNEL (crossbeam-channel, 100% thread-safe!) ---
pub struct StrivChannel {
    tx: Sender<i64>,
    rx: Receiver<i64>,
}
#[unsafe(no_mangle)]
pub extern "C" fn striv_channel_new() -> *mut StrivChannel {
    let (tx, rx) = unbounded();
    Box::into_raw(Box::new(StrivChannel { tx, rx }))
}
#[unsafe(no_mangle)]
pub extern "C" fn striv_channel_send(p: *const StrivChannel, v: i64) -> i64 {
    let _ = unsafe { (*p).tx.send(v) };
    0
}
#[unsafe(no_mangle)]
pub extern "C" fn striv_channel_recv(p: *const StrivChannel) -> i64 {
    unsafe { (*p).rx.recv().unwrap_or(0) }
}
#[unsafe(no_mangle)]
pub extern "C" fn striv_channel_try_recv(p: *const StrivChannel) -> i64 {
    unsafe { (*p).rx.try_recv().unwrap_or(-1) }
}
#[unsafe(no_mangle)]
pub extern "C" fn striv_channel_free(p: *mut StrivChannel) {
    if !p.is_null() {
        unsafe { drop(Box::from_raw(p)) };
    }
}

// --- 10b. ATOMIC (Lock-free multi-core counter) ---
#[unsafe(no_mangle)]
pub extern "C" fn striv_atomic_new(init: i64) -> *mut AtomicI64 {
    Box::into_raw(Box::new(AtomicI64::new(init)))
}
#[unsafe(no_mangle)]
pub extern "C" fn striv_atomic_add(p: *const AtomicI64, val: i64) -> i64 {
    unsafe { (*p).fetch_add(val, Ordering::SeqCst) + val }
}
#[unsafe(no_mangle)]
pub extern "C" fn striv_atomic_sub(p: *const AtomicI64, val: i64) -> i64 {
    unsafe { (*p).fetch_sub(val, Ordering::SeqCst) - val }
}
#[unsafe(no_mangle)]
pub extern "C" fn striv_atomic_get(p: *const AtomicI64) -> i64 {
    unsafe { (*p).load(Ordering::SeqCst) }
}
#[unsafe(no_mangle)]
pub extern "C" fn striv_atomic_set(p: *const AtomicI64, val: i64) -> i64 {
    unsafe { (*p).store(val, Ordering::SeqCst) };
    0
}
#[unsafe(no_mangle)]
pub extern "C" fn striv_atomic_free(p: *mut AtomicI64) {
    if !p.is_null() {
        unsafe { drop(Box::from_raw(p)) };
    }
}

// --- 10c. TASK-GROUP FOR `concurrent { spawn ... }` (Rayon Thread-Pool) ---
pub struct TaskGroup {
    jobs: Vec<(usize, Vec<i64>)>,
}
#[unsafe(no_mangle)]
pub extern "C" fn striv_group_begin() -> *mut TaskGroup {
    Box::into_raw(Box::new(TaskGroup { jobs: Vec::new() }))
}
#[unsafe(no_mangle)]
pub extern "C" fn striv_group_spawn(
    g_ptr: *mut TaskGroup,
    fn_addr: usize,
    args_ptr: *const i64,
    arg_count: i64,
) {
    let group = unsafe { &mut *g_ptr };
    let args = if arg_count > 0 {
        unsafe { std::slice::from_raw_parts(args_ptr, arg_count as usize).to_vec() }
    } else {
        Vec::new()
    };
    group.jobs.push((fn_addr, args));
}
#[unsafe(no_mangle)]
pub extern "C" fn striv_group_wait(g_ptr: *mut TaskGroup) {
    if g_ptr.is_null() {
        return;
    }
    let group = unsafe { Box::from_raw(g_ptr) };
    rayon::scope(|s| {
        for (fn_addr, args) in group.jobs {
            s.spawn(move |_| unsafe {
                call_jit_fn_dynamic(fn_addr, &args);
            });
        }
    });
}
unsafe fn call_jit_fn_dynamic(addr: usize, a: &[i64]) -> i64 {
    let ptr = addr as *const ();
    match a.len() {
        0 => unsafe { std::mem::transmute::<*const (), fn() -> i64>(ptr)() },
        1 => unsafe { std::mem::transmute::<*const (), fn(i64) -> i64>(ptr)(a[0]) },
        2 => unsafe { std::mem::transmute::<*const (), fn(i64, i64) -> i64>(ptr)(a[0], a[1]) },
        3 => unsafe {
            std::mem::transmute::<*const (), fn(i64, i64, i64) -> i64>(ptr)(a[0], a[1], a[2])
        },
        4 => unsafe {
            std::mem::transmute::<*const (), fn(i64, i64, i64, i64) -> i64>(ptr)(
                a[0], a[1], a[2], a[3],
            )
        },
        5 => unsafe {
            std::mem::transmute::<*const (), fn(i64, i64, i64, i64, i64) -> i64>(ptr)(
                a[0], a[1], a[2], a[3], a[4],
            )
        },
        _ => panic!("spawn currently supports up to 5 parameters"),
    }
}

// --- 11. VEC3 & MAT4 (3D Math) ---
pub struct Vec3D {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}
#[unsafe(no_mangle)]
pub extern "C" fn striv_vec3_new(x: f64, y: f64, z: f64) -> *mut Vec3D {
    Box::into_raw(Box::new(Vec3D { x, y, z }))
}
#[unsafe(no_mangle)]
pub extern "C" fn striv_vec3_dot(a: *const Vec3D, b: *const Vec3D) -> f64 {
    unsafe { (*a).x * (*b).x + (*a).y * (*b).y + (*a).z * (*b).z }
}
#[unsafe(no_mangle)]
pub extern "C" fn striv_vec3_len(a: *const Vec3D) -> f64 {
    unsafe { ((*a).x * (*a).x + (*a).y * (*a).y + (*a).z * (*a).z).sqrt() }
}
#[unsafe(no_mangle)]
pub extern "C" fn striv_vec3_free(p: *mut Vec3D) {
    if !p.is_null() {
        unsafe { drop(Box::from_raw(p)) };
    }
}

pub struct Mat4D(pub [f64; 16]);
#[unsafe(no_mangle)]
pub extern "C" fn striv_mat4_identity() -> *mut Mat4D {
    let mut m = [0.0; 16];
    m[0] = 1.0;
    m[5] = 1.0;
    m[10] = 1.0;
    m[15] = 1.0;
    Box::into_raw(Box::new(Mat4D(m)))
}
#[unsafe(no_mangle)]
pub extern "C" fn striv_mat4_get(p: *const Mat4D, idx: i64) -> f64 {
    let i = idx.clamp(0, 15) as usize;
    unsafe { (*p).0[i] }
}
#[unsafe(no_mangle)]
pub extern "C" fn striv_mat4_set(p: *mut Mat4D, idx: i64, val: f64) -> i64 {
    let i = idx.clamp(0, 15) as usize;
    unsafe { (*p).0[i] = val };
    0
}
#[unsafe(no_mangle)]
pub extern "C" fn striv_mat4_free(p: *mut Mat4D) {
    if !p.is_null() {
        unsafe { drop(Box::from_raw(p)) };
    }
}

/// Central table of built-in stdlib methods: (Striv-Name, Symbol-Ptr, ABI-Sig, Mutates-Self?)
pub fn std_collection_symbols() -> Vec<(&'static str, *const u8, &'static str, bool)> {
    vec![
        ("Ecs::new", crate::ecs::striv_ecs_new as *const u8, "->i", false),
        ("Ecs::spawn", crate::ecs::striv_ecs_spawn as *const u8, "i->i", false),
        ("Ecs::remove_entity", crate::ecs::striv_ecs_remove_entity as *const u8, "ii->i", false),
        ("String::trim", striv_str_trim as *const u8, "i->i", false),
        ("String::contains", striv_str_contains as *const u8, "ii->i", false),
        ("String::starts_with", striv_str_starts_with as *const u8, "ii->i", false),
        ("String::replace", striv_str_replace as *const u8, "iii->i", false),
        ("String::to_int", striv_str_to_int as *const u8, "i->i", false),
        ("String::to_float", striv_str_to_float as *const u8, "i->f", false),
        ("fs::read_text", striv_fs_read_text as *const u8, "i->i", false),
        ("fs::write_text", striv_fs_write_text as *const u8, "ii->i", false),
        // Constructors
        ("Deque::new", striv_deque_new as *const u8, "->i", false),
        ("Map::new", striv_map_new as *const u8, "->i", false),
        ("StrMap::new", striv_strmap_new as *const u8, "->i", false),
        ("BTree::new", striv_btree_new as *const u8, "->i", false),
        ("Set::new", striv_set_new as *const u8, "->i", false),
        ("Heap::new", striv_heap_new as *const u8, "->i", false),
        ("Grid::new", striv_grid_new as *const u8, "iii->i", false),
        ("Bytes::new", striv_bytes_new as *const u8, "->i", false),
        (
            "Bytes::read_file",
            striv_bytes_read_file as *const u8,
            "i->i",
            false,
        ),
        ("Rng::new", striv_rng_new as *const u8, "i->i", false),
        ("BitSet::new", striv_bitset_new as *const u8, "i->i", false),
        ("Channel::new", striv_channel_new as *const u8, "->i", false),
        ("Atomic::new", striv_atomic_new as *const u8, "i->i", false),
        ("Vec3::new", striv_vec3_new as *const u8, "fff->i", false),
        (
            "Mat4::identity",
            striv_mat4_identity as *const u8,
            "->i",
            false,
        ),
        // Methods
        (
            "Deque__push_back",
            striv_deque_push_back as *const u8,
            "ii->i",
            true,
        ),
        (
            "Deque__push_front",
            striv_deque_push_front as *const u8,
            "ii->i",
            true,
        ),
        (
            "Deque__pop_front",
            striv_deque_pop_front as *const u8,
            "i->i",
            true,
        ),
        (
            "Deque__pop_back",
            striv_deque_pop_back as *const u8,
            "i->i",
            true,
        ),
        ("Deque__len", striv_deque_len as *const u8, "i->i", false),
        ("Map__insert", striv_map_insert as *const u8, "iii->i", true),
        ("Map__get", striv_map_get as *const u8, "ii->i", false),
        (
            "Map__contains",
            striv_map_contains as *const u8,
            "ii->i",
            false,
        ),
        ("Map__remove", striv_map_remove as *const u8, "ii->i", true),
        ("Map__len", striv_map_len as *const u8, "i->i", false),
        (
            "StrMap__insert",
            striv_strmap_insert as *const u8,
            "iii->i",
            true,
        ),
        ("StrMap__get", striv_strmap_get as *const u8, "ii->i", false),
        (
            "StrMap__contains",
            striv_strmap_contains as *const u8,
            "ii->i",
            false,
        ),
        (
            "BTree__insert",
            striv_btree_insert as *const u8,
            "iii->i",
            true,
        ),
        ("BTree__get", striv_btree_get as *const u8, "ii->i", false),
        (
            "BTree__first_key",
            striv_btree_first_key as *const u8,
            "i->i",
            false,
        ),
        (
            "BTree__last_key",
            striv_btree_last_key as *const u8,
            "i->i",
            false,
        ),
        ("Set__insert", striv_set_insert as *const u8, "ii->i", true),
        (
            "Set__contains",
            striv_set_contains as *const u8,
            "ii->i",
            false,
        ),
        ("Set__remove", striv_set_remove as *const u8, "ii->i", true),
        ("Set__len", striv_set_len as *const u8, "i->i", false),
        ("Heap__push", striv_heap_push as *const u8, "ii->i", true),
        ("Heap__pop", striv_heap_pop as *const u8, "i->i", true),
        ("Heap__peek", striv_heap_peek as *const u8, "i->i", false),
        ("Heap__len", striv_heap_len as *const u8, "i->i", false),
        ("Grid__get", striv_grid_get as *const u8, "iii->i", false),
        ("Grid__set", striv_grid_set as *const u8, "iiii->i", true),
        (
            "Grid__in_bounds",
            striv_grid_in_bounds as *const u8,
            "iii->i",
            false,
        ),
        ("Bytes__push", striv_bytes_push as *const u8, "ii->i", true),
        ("Bytes__get", striv_bytes_get as *const u8, "ii->i", false),
        ("Bytes__len", striv_bytes_len as *const u8, "i->i", false),
        ("Rng__range", striv_rng_range as *const u8, "iii->i", true),
        ("BitSet__set", striv_bitset_set as *const u8, "ii->i", true),
        ("BitSet__get", striv_bitset_get as *const u8, "ii->i", false),
        (
            "BitSet__clear",
            striv_bitset_clear as *const u8,
            "ii->i",
            true,
        ),
        (
            "Channel__send",
            striv_channel_send as *const u8,
            "ii->i",
            false,
        ),
        (
            "Channel__recv",
            striv_channel_recv as *const u8,
            "i->i",
            false,
        ),
        (
            "Channel__try_recv",
            striv_channel_try_recv as *const u8,
            "i->i",
            false,
        ),
        ("Atomic__add", striv_atomic_add as *const u8, "ii->i", false),
        ("Atomic__sub", striv_atomic_sub as *const u8, "ii->i", false),
        ("Atomic__get", striv_atomic_get as *const u8, "i->i", false),
        ("Atomic__set", striv_atomic_set as *const u8, "ii->i", false),
        ("Vec3__dot", striv_vec3_dot as *const u8, "ii->f", false),
        ("Vec3__len", striv_vec3_len as *const u8, "i->f", false),
        ("Mat4__get", striv_mat4_get as *const u8, "ii->f", false),
        ("Mat4__set", striv_mat4_set as *const u8, "iif->i", true),
    ]
}

/// Central table of automatic Drop functions per type name!
pub fn std_drop_symbols() -> Vec<(&'static str, *const u8)> {
    vec![
        ("Ecs", crate::ecs::striv_ecs_free as *const u8),
        ("Deque", striv_deque_free as *const u8),
        ("Map", striv_map_free as *const u8),
        ("StrMap", striv_strmap_free as *const u8),
        ("BTree", striv_btree_free as *const u8),
        ("Set", striv_set_free as *const u8),
        ("Heap", striv_heap_free as *const u8),
        ("Grid", striv_grid_free as *const u8),
        ("Bytes", striv_bytes_free as *const u8),
        ("Rng", striv_rng_free as *const u8),
        ("BitSet", striv_bitset_free as *const u8),
        ("Channel", striv_channel_free as *const u8),
        ("Atomic", striv_atomic_free as *const u8),
        ("Vec3", striv_vec3_free as *const u8),
        ("Mat4", striv_mat4_free as *const u8),
    ]
}

// --- 12. RAW STRUCT VEC (Flat ECS memory for `Vec<Struct>`) ---
pub struct RawStructVec {
    pub data: Vec<u8>,
    pub elem_size: usize,
    pub len: usize,
}

#[unsafe(no_mangle)]
pub extern "C" fn striv_svec_new(elem_size: i64) -> *mut RawStructVec {
    Box::into_raw(Box::new(RawStructVec {
        data: Vec::new(),
        elem_size: elem_size.max(8) as usize,
        len: 0,
    }))
}

#[unsafe(no_mangle)]
pub extern "C" fn striv_svec_push(v_ptr: *mut RawStructVec, src_ptr: *const u8) -> i64 {
    let v = unsafe { &mut *v_ptr };
    let bytes = unsafe { std::slice::from_raw_parts(src_ptr, v.elem_size) };
    v.data.extend_from_slice(bytes);
    v.len += 1;
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn striv_svec_get_ptr(v_ptr: *mut RawStructVec, idx: i64) -> *mut u8 {
    let v = unsafe { &mut *v_ptr };
    assert!(
        (idx as usize) < v.len,
        "Vec index {} out of bounds (len {})",
        idx,
        v.len
    );
    unsafe { v.data.as_mut_ptr().add((idx as usize) * v.elem_size) }
}

#[unsafe(no_mangle)]
pub extern "C" fn striv_svec_len(v_ptr: *const RawStructVec) -> i64 {
    unsafe { (*v_ptr).len as i64 }
}

#[unsafe(no_mangle)]
pub extern "C" fn striv_svec_as_bytes_ptr(v_ptr: *const RawStructVec) -> i64 {
    unsafe { (*v_ptr).data.as_ptr() as i64 }
}

#[unsafe(no_mangle)]
pub extern "C" fn striv_svec_byte_len(v_ptr: *const RawStructVec) -> i64 {
    unsafe { (*v_ptr).data.len() as i64 }
}

#[unsafe(no_mangle)]
pub extern "C" fn striv_svec_to_bytes(v_ptr: *const RawStructVec) -> *mut Vec<u8> {
    let data = unsafe { (*v_ptr).data.clone() };
    Box::into_raw(Box::new(data))
}

#[unsafe(no_mangle)]
pub extern "C" fn striv_str_trim(s: *const String) -> *mut String {
    Box::into_raw(Box::new(unsafe { (*s).trim().to_string() }))
}

#[unsafe(no_mangle)]
pub extern "C" fn striv_str_contains(s: *const String, pat: *const String) -> i64 {
    unsafe { (*s).contains((*pat).as_str()) as i64 }
}

#[unsafe(no_mangle)]
pub extern "C" fn striv_str_starts_with(s: *const String, pat: *const String) -> i64 {
    unsafe { (*s).starts_with((*pat).as_str()) as i64 }
}

#[unsafe(no_mangle)]
pub extern "C" fn striv_str_replace(
    s: *const String,
    a: *const String,
    b: *const String,
) -> *mut String {
    Box::into_raw(Box::new(unsafe {
        (*s).replace((*a).as_str(), (*b).as_str())
    }))
}

#[unsafe(no_mangle)]
pub extern "C" fn striv_str_to_int(s: *const String) -> i64 {
    unsafe { (*s).trim().parse::<i64>().unwrap_or(0) }
}

#[unsafe(no_mangle)]
pub extern "C" fn striv_str_to_float(s: *const String) -> f64 {
    unsafe { (*s).trim().parse::<f64>().unwrap_or(0.0) }
}

#[unsafe(no_mangle)]
pub extern "C" fn striv_fs_read_text(path: *const String) -> *mut String {
    let p = unsafe { &*path };
    let content = std::fs::read_to_string(p).unwrap_or_default();
    Box::into_raw(Box::new(content))
}

#[unsafe(no_mangle)]
pub extern "C" fn striv_fs_write_text(path: *const String, data: *const String) -> i64 {
    let p = unsafe { &*path };
    let d = unsafe { &*data };
    if let Some(parent) = std::path::Path::new(p).parent() {
        if !parent.as_os_str().is_empty() {
            let _ = std::fs::create_dir_all(parent);
        }
    }
    std::fs::write(p, d).is_ok() as i64
}

#[unsafe(no_mangle)]
pub extern "C" fn striv_bytes_write_u32(p: *mut Vec<u8>, v: i64) -> i64 {
    unsafe {
        (*p).extend_from_slice(&(v as u32).to_le_bytes());
    }
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn striv_bytes_write_i64(p: *mut Vec<u8>, v: i64) -> i64 {
    unsafe {
        (*p).extend_from_slice(&v.to_le_bytes());
    }
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn striv_bytes_write_f64(p: *mut Vec<u8>, v: f64) -> i64 {
    unsafe {
        (*p).extend_from_slice(&v.to_le_bytes());
    }
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn striv_bytes_read_u32(p: *const Vec<u8>, offset: i64) -> i64 {
    let b = unsafe { &*p };
    let off = offset as usize;
    if off + 4 > b.len() {
        return -1;
    }
    u32::from_le_bytes(b[off..off + 4].try_into().unwrap()) as i64
}

#[unsafe(no_mangle)]
pub extern "C" fn striv_bytes_read_i64(p: *const Vec<u8>, offset: i64) -> i64 {
    let b = unsafe { &*p };
    let off = offset as usize;
    if off + 8 > b.len() {
        return 0;
    }
    i64::from_le_bytes(b[off..off + 8].try_into().unwrap())
}

#[unsafe(no_mangle)]
pub extern "C" fn striv_bytes_read_f64(p: *const Vec<u8>, offset: i64) -> f64 {
    let b = unsafe { &*p };
    let off = offset as usize;
    if off + 8 > b.len() {
        return 0.0;
    }
    f64::from_le_bytes(b[off..off + 8].try_into().unwrap())
}

#[unsafe(no_mangle)]
pub extern "C" fn striv_pack_struct(
    struct_ptr: *const u8,
    layout_ptr: *const u32,
    full: i64,
) -> *mut Vec<u8> {
    unsafe {
        let count = *layout_ptr as usize;
        let sync_mask = (*layout_ptr.add(1) as u64) | ((*layout_ptr.add(2) as u64) << 32);
        let dirty_mask = if full != 0 {
            sync_mask
        } else {
            *(struct_ptr as *const u64) & sync_mask
        };
        let mut out = Vec::with_capacity(64);
        out.extend_from_slice(&dirty_mask.to_le_bytes());
        for i in 0..count {
            if (dirty_mask & (1u64 << i)) != 0 {
                let off = *layout_ptr.add(3 + i * 2) as usize;
                let sz = *layout_ptr.add(3 + i * 2 + 1) as usize;
                out.extend_from_slice(std::slice::from_raw_parts(struct_ptr.add(off), sz));
            }
        }
        Box::into_raw(Box::new(out))
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn striv_apply_struct(
    struct_ptr: *mut u8,
    layout_ptr: *const u32,
    bytes_ptr: *const Vec<u8>,
) -> i64 {
    unsafe {
        let pkt = &*bytes_ptr;
        if pkt.len() < 8 {
            return 0;
        }
        let count = *layout_ptr as usize;
        let mask = u64::from_le_bytes(pkt[0..8].try_into().unwrap());
        let mut cursor = 8usize;
        for i in 0..count {
            if (mask & (1u64 << i)) != 0 {
                let off = *layout_ptr.add(3 + i * 2) as usize;
                let sz = *layout_ptr.add(3 + i * 2 + 1) as usize;
                if cursor + sz <= pkt.len() {
                    std::ptr::copy_nonoverlapping(
                        pkt.as_ptr().add(cursor),
                        struct_ptr.add(off),
                        sz,
                    );
                }
                cursor += sz;
            }
        }
        0
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn striv_svec_clear(v_ptr: *mut RawStructVec) -> i64 {
    let v = unsafe { &mut *v_ptr };
    v.data.clear();
    v.len = 0;
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn striv_svec_free(v_ptr: *mut RawStructVec) {
    if !p_is_null(v_ptr) {
        unsafe { drop(Box::from_raw(v_ptr)) };
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn striv_str_clone(p: *const String) -> *mut String {
    let cloned = if p.is_null() {
        String::new()
    } else {
        unsafe { (*p).clone() }
    };
    Box::into_raw(Box::new(cloned))
}

#[unsafe(no_mangle)]
pub extern "C" fn striv_bytes_clone(p: *const Vec<u8>) -> *mut Vec<u8> {
    let cloned = if p.is_null() {
        Vec::new()
    } else {
        unsafe { (*p).clone() }
    };
    Box::into_raw(Box::new(cloned))
}

#[unsafe(no_mangle)]
pub extern "C" fn striv_vec_clone(p: *const Vec<i64>) -> *mut Vec<i64> {
    let cloned = if p.is_null() {
        Vec::new()
    } else {
        unsafe { (*p).clone() }
    };
    Box::into_raw(Box::new(cloned))
}

#[unsafe(no_mangle)]
pub extern "C" fn striv_svec_clone(p: *const RawStructVec) -> *mut RawStructVec {
    if p.is_null() {
        return Box::into_raw(Box::new(RawStructVec {
            elem_size: 8,
            len: 0,
            data: Vec::new(),
        }));
    }
    let src = unsafe { &*p };
    Box::into_raw(Box::new(RawStructVec {
        elem_size: src.elem_size,
        len: src.len,
        data: src.data.clone(),
    }))
}

#[inline]
fn p_is_null<T>(p: *mut T) -> bool {
    p.is_null()
}

// --- 13. SHARED BOX (`shared x = Struct { ... }` + `lock x { ... }`) ---
pub struct SharedBox {
    mutex: RawMutex,
    data: std::cell::UnsafeCell<Vec<u8>>,
}
unsafe impl Send for SharedBox {}
unsafe impl Sync for SharedBox {}

#[unsafe(no_mangle)]
pub extern "C" fn striv_shared_new(src_ptr: *const u8, byte_size: i64) -> *mut SharedBox {
    let slice = unsafe { std::slice::from_raw_parts(src_ptr, byte_size as usize) };
    Box::into_raw(Box::new(SharedBox {
        mutex: RawMutex::INIT,
        data: std::cell::UnsafeCell::new(slice.to_vec()),
    }))
}

#[unsafe(no_mangle)]
pub extern "C" fn striv_shared_lock(p: *const SharedBox) -> *mut u8 {
    let sb = unsafe { &*p };
    sb.mutex.lock();
    unsafe { (*sb.data.get()).as_mut_ptr() }
}

#[unsafe(no_mangle)]
pub extern "C" fn striv_shared_unlock(p: *const SharedBox) {
    let sb = unsafe { &*p };
    unsafe { sb.mutex.unlock() };
}

#[unsafe(no_mangle)]
pub extern "C" fn striv_shared_free(p: *mut SharedBox) {
    if !p.is_null() {
        unsafe { drop(Box::from_raw(p)) };
    }
}

// --- 14. PARALLEL FOR (`parallel for [mut] e in list`) ---
#[unsafe(no_mangle)]
pub extern "C" fn striv_par_for_svec(
    v_ptr: *mut RawStructVec,
    body_fn_addr: usize,
    caps_ptr: *const i64,
    cap_count: i64,
) {
    let v = unsafe { &mut *v_ptr };
    if v.len == 0 {
        return;
    }
    let caps: &[i64] = if cap_count > 0 {
        unsafe { std::slice::from_raw_parts(caps_ptr, cap_count as usize) }
    } else {
        &[]
    };
    let elem_size = v.elem_size;
    v.data
        .par_chunks_exact_mut(elem_size)
        .for_each(|chunk| unsafe {
            let elem_addr = chunk.as_mut_ptr() as i64;
            call_par_body(body_fn_addr, elem_addr, caps);
        });
}

#[unsafe(no_mangle)]
pub extern "C" fn striv_par_for_vec(
    v_ptr: *mut Vec<i64>,
    body_fn_addr: usize,
    caps_ptr: *const i64,
    cap_count: i64,
) {
    let v = unsafe { &mut *v_ptr };
    let caps: &[i64] = if cap_count > 0 {
        unsafe { std::slice::from_raw_parts(caps_ptr, cap_count as usize) }
    } else {
        &[]
    };
    v.par_iter_mut().for_each(|item| unsafe {
        call_par_body(body_fn_addr, *item, caps);
    });
}

unsafe fn call_par_body(addr: usize, elem: i64, c: &[i64]) {
    let ptr = addr as *const ();
    match c.len() {
        0 => unsafe {
            std::mem::transmute::<*const (), fn(i64) -> i64>(ptr)(elem);
        },
        1 => unsafe {
            std::mem::transmute::<*const (), fn(i64, i64) -> i64>(ptr)(elem, c[0]);
        },
        2 => unsafe {
            std::mem::transmute::<*const (), fn(i64, i64, i64) -> i64>(ptr)(elem, c[0], c[1]);
        },
        3 => unsafe {
            std::mem::transmute::<*const (), fn(i64, i64, i64, i64) -> i64>(ptr)(
                elem, c[0], c[1], c[2],
            );
        },
        4 => unsafe {
            std::mem::transmute::<*const (), fn(i64, i64, i64, i64, i64) -> i64>(ptr)(
                elem, c[0], c[1], c[2], c[3],
            );
        },
        _ => panic!("parallel for currently supports up to 4 captured variables"),
    }
}

// --- 15. SELECT (`select { msg = ch.recv() => { ... } timeout(ms) => { ... } }`) ---
#[unsafe(no_mangle)]
pub extern "C" fn striv_select_recv(
    chans_ptr: *const *const StrivChannel,
    count: i64,
    timeout_ms: i64,
    out_val: *mut i64,
) -> i64 {
    let slice = unsafe { std::slice::from_raw_parts(chans_ptr, count as usize) };
    let mut sel = Select::new();
    for &ch_ptr in slice {
        let ch = unsafe { &*ch_ptr };
        sel.recv(&ch.rx);
    }

    let oper = if timeout_ms >= 0 {
        match sel.select_timeout(Duration::from_millis(timeout_ms as u64)) {
            Ok(op) => op,
            Err(_) => return -1,
        }
    } else {
        sel.select()
    };

    let idx = oper.index();
    let ch = unsafe { &*slice[idx] };
    let val = oper.recv(&ch.rx).unwrap_or(0);
    unsafe {
        *out_val = val;
    }
    idx as i64
}
