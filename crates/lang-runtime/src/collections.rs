use std::collections::{BTreeMap, BinaryHeap, HashMap, HashSet, VecDeque};
use std::sync::mpsc::{self, Receiver, Sender};

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

// --- 10. CHANNEL (mpsc::channel::<i64>()) ---
pub struct StrivChannel {
    tx: Sender<i64>,
    rx: Receiver<i64>,
}
#[unsafe(no_mangle)]
pub extern "C" fn striv_channel_new() -> *mut StrivChannel {
    let (tx, rx) = mpsc::channel();
    Box::into_raw(Box::new(StrivChannel { tx, rx }))
}
#[unsafe(no_mangle)]
pub extern "C" fn striv_channel_send(p: *const StrivChannel, v: i64) -> i64 {
    unsafe { (*p).tx.send(v).is_ok() as i64 }
}
#[unsafe(no_mangle)]
pub extern "C" fn striv_channel_recv(p: *const StrivChannel) -> i64 {
    unsafe { (*p).rx.try_recv().unwrap_or(0) }
}
#[unsafe(no_mangle)]
pub extern "C" fn striv_channel_free(p: *mut StrivChannel) {
    if !p.is_null() {
        unsafe { drop(Box::from_raw(p)) };
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
        ("Vec3::new", striv_vec3_new as *const u8, "fff->i", false),
        (
            "Mat4::identity",
            striv_mat4_identity as *const u8,
            "->i",
            false,
        ),
        // Methods
        ("Deque__push_back", striv_deque_push_back as *const u8, "ii->i", true),
        ("Deque__push_front", striv_deque_push_front as *const u8, "ii->i", true),
        ("Deque__pop_front", striv_deque_pop_front as *const u8, "i->i", true),
        ("Deque__pop_back", striv_deque_pop_back as *const u8, "i->i", true),
        ("Deque__len", striv_deque_len as *const u8, "i->i", false),
        ("Map__insert", striv_map_insert as *const u8, "iii->i", true),
        ("Map__get", striv_map_get as *const u8, "ii->i", false),
        ("Map__contains", striv_map_contains as *const u8, "ii->i", false),
        ("Map__remove", striv_map_remove as *const u8, "ii->i", true),
        ("Map__len", striv_map_len as *const u8, "i->i", false),
        ("StrMap__insert", striv_strmap_insert as *const u8, "iii->i", true),
        ("StrMap__get", striv_strmap_get as *const u8, "ii->i", false),
        (
            "StrMap__contains",
            striv_strmap_contains as *const u8,
            "ii->i",
            false,
        ),
        ("BTree__insert", striv_btree_insert as *const u8, "iii->i", true),
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
        ("Set__contains", striv_set_contains as *const u8, "ii->i", false),
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
        ("BitSet__clear", striv_bitset_clear as *const u8, "ii->i", true),
        ("Channel__send", striv_channel_send as *const u8, "ii->i", false),
        ("Channel__recv", striv_channel_recv as *const u8, "i->i", false),
        ("Vec3__dot", striv_vec3_dot as *const u8, "ii->f", false),
        ("Vec3__len", striv_vec3_len as *const u8, "i->f", false),
        ("Mat4__get", striv_mat4_get as *const u8, "ii->f", false),
        ("Mat4__set", striv_mat4_set as *const u8, "iif->i", true),
    ]
}

/// Central table of automatic Drop functions per type name!
pub fn std_drop_symbols() -> Vec<(&'static str, *const u8)> {
    vec![
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

#[inline]
fn p_is_null<T>(p: *mut T) -> bool {
    p.is_null()
}
