use rayon::prelude::*;
use std::collections::{HashMap, HashSet};

pub struct ComponentPool {
    pub elem_size: usize,
    pub entities: Vec<i64>,
    pub data: Vec<u8>,
    pub index_of: HashMap<i64, usize>,
    pub changed: HashSet<i64>,
}

impl ComponentPool {
    fn new(elem_size: usize) -> Self {
        Self {
            elem_size: elem_size.max(1),
            entities: Vec::new(),
            data: Vec::new(),
            index_of: HashMap::new(),
            changed: HashSet::new(),
        }
    }

    fn upsert(&mut self, entity: i64, bytes: &[u8]) -> bool {
        self.changed.insert(entity);
        if let Some(&idx) = self.index_of.get(&entity) {
            let off = idx * self.elem_size;
            self.data[off..off + self.elem_size].copy_from_slice(bytes);
            false
        } else {
            let idx = self.entities.len();
            self.entities.push(entity);
            self.data.extend_from_slice(bytes);
            self.index_of.insert(entity, idx);
            true
        }
    }

    fn remove(&mut self, entity: i64) -> bool {
        if let Some(idx) = self.index_of.remove(&entity) {
            let last_idx = self.entities.len() - 1;
            let sz = self.elem_size;
            if idx != last_idx {
                let moved_ent = self.entities[last_idx];
                self.entities[idx] = moved_ent;
                self.data
                    .copy_within(last_idx * sz..(last_idx + 1) * sz, idx * sz);
                self.index_of.insert(moved_ent, idx);
            }
            self.entities.pop();
            self.data.truncate(last_idx * sz);
            self.changed.remove(&entity);
            true
        } else {
            false
        }
    }

    #[inline]
    fn get_ptr(&mut self, entity: i64) -> Option<*mut u8> {
        let &idx = self.index_of.get(&entity)?;
        Some(unsafe { self.data.as_mut_ptr().add(idx * self.elem_size) })
    }
}

pub struct ArchetypeTable {
    pub signature: Vec<i64>,
    pub entities: Vec<i64>,
    pub columns: HashMap<i64, Vec<u8>>,
    pub elem_sizes: HashMap<i64, usize>,
    pub changed: HashMap<i64, HashSet<i64>>,
}

impl ArchetypeTable {
    fn new(signature: Vec<i64>, elem_sizes: HashMap<i64, usize>) -> Self {
        let mut columns = HashMap::with_capacity(signature.len());
        let mut changed = HashMap::with_capacity(signature.len());
        for &tid in &signature {
            columns.insert(tid, Vec::new());
            changed.insert(tid, HashSet::new());
        }
        Self {
            signature,
            entities: Vec::new(),
            columns,
            elem_sizes,
            changed,
        }
    }

    #[inline]
    fn contains(&self, tid: i64) -> bool {
        self.signature.binary_search(&tid).is_ok()
    }

    #[inline]
    fn get_ptr(&mut self, row: usize, tid: i64) -> Option<*mut u8> {
        let sz = *self.elem_sizes.get(&tid)?;
        let col = self.columns.get_mut(&tid)?;
        Some(unsafe { col.as_mut_ptr().add(row * sz) })
    }

    fn set_in_row(&mut self, row: usize, entity: i64, tid: i64, bytes: &[u8]) {
        let sz = self.elem_sizes[&tid];
        let col = self.columns.get_mut(&tid).unwrap();
        let off = row * sz;
        col[off..off + sz].copy_from_slice(bytes);
        if let Some(ch) = self.changed.get_mut(&tid) {
            ch.insert(entity);
        }
    }

    fn push_row(&mut self, entity: i64, comp_bytes: &[(i64, &[u8])]) -> usize {
        let row = self.entities.len();
        self.entities.push(entity);
        for &(tid, bytes) in comp_bytes {
            let col = self.columns.get_mut(&tid).unwrap();
            col.extend_from_slice(bytes);
        }
        row
    }

    /// Swap-removes `row`, returning `Some(moved_entity)` if another entity was moved into `row`.
    fn swap_remove_row(&mut self, row: usize) -> Option<i64> {
        let last_row = self.entities.len() - 1;
        let removed_ent = self.entities[row];
        let moved = if row != last_row {
            let moved_ent = self.entities[last_row];
            self.entities[row] = moved_ent;
            Some(moved_ent)
        } else {
            None
        };
        self.entities.pop();

        for &tid in &self.signature {
            let sz = self.elem_sizes[&tid];
            let col = self.columns.get_mut(&tid).unwrap();
            if row != last_row {
                col.copy_within(last_row * sz..(last_row + 1) * sz, row * sz);
            }
            col.truncate(last_row * sz);
            if let Some(ch) = self.changed.get_mut(&tid) {
                ch.remove(&removed_ent);
            }
        }
        moved
    }
}

pub struct StrivEcs {
    next_entity: i64,
    archetypes: Vec<ArchetypeTable>,
    sig_to_arch: HashMap<Vec<i64>, usize>,
    entity_loc: HashMap<i64, (usize, usize)>,
    sparse_types: HashSet<i64>,
    sparse_pools: HashMap<i64, ComponentPool>,
    query_cache: HashMap<(Vec<i64>, Vec<i64>), Vec<usize>>,
    relations: HashMap<(i64, i64), Vec<i64>>,
    added_events: HashMap<i64, HashSet<i64>>,
    removed_events: HashMap<i64, HashSet<i64>>,
    pub parents: HashMap<i64, i64>,
    pub groups: HashMap<i64, i64>,
}

impl StrivEcs {
    fn new() -> Self {
        let empty = ArchetypeTable::new(Vec::new(), HashMap::new());
        let mut sig_to_arch = HashMap::new();
        sig_to_arch.insert(Vec::new(), 0);
        Self {
            next_entity: 1,
            archetypes: vec![empty],
            sig_to_arch,
            entity_loc: HashMap::new(),
            sparse_types: HashSet::new(),
            sparse_pools: HashMap::new(),
            query_cache: HashMap::new(),
            relations: HashMap::new(),
            added_events: HashMap::new(),
            removed_events: HashMap::new(),
            parents: HashMap::new(),
            groups: HashMap::new(),
        }
    }

    fn get_or_create_archetype(
        &mut self,
        sig: Vec<i64>,
        elem_sizes: HashMap<i64, usize>,
    ) -> usize {
        if let Some(&id) = self.sig_to_arch.get(&sig) {
            return id;
        }
        let id = self.archetypes.len();
        self.sig_to_arch.insert(sig.clone(), id);
        self.archetypes.push(ArchetypeTable::new(sig, elem_sizes));
        self.query_cache.clear();
        id
    }

    fn set_dense(&mut self, entity: i64, tid: i64, elem_size: usize, slice: &[u8]) {
        if let Some(&(arch_id, row)) = self.entity_loc.get(&entity) {
            if self.archetypes[arch_id].contains(tid) {
                self.archetypes[arch_id].set_in_row(row, entity, tid, slice);
                return;
            }
            // Migrate entity to new archetype with `tid` added
            let mut new_sig = self.archetypes[arch_id].signature.clone();
            let pos = new_sig.binary_search(&tid).unwrap_err();
            new_sig.insert(pos, tid);

            let mut new_sizes = self.archetypes[arch_id].elem_sizes.clone();
            new_sizes.insert(tid, elem_size);
            let new_arch_id = self.get_or_create_archetype(new_sig, new_sizes);

            let old_sig = self.archetypes[arch_id].signature.clone();
            let mut copied: Vec<(i64, Vec<u8>)> = Vec::with_capacity(old_sig.len() + 1);
            let mut preserved_changed: Vec<i64> = Vec::new();
            for &old_tid in &old_sig {
                let sz = self.archetypes[arch_id].elem_sizes[&old_tid];
                let col = &self.archetypes[arch_id].columns[&old_tid];
                copied.push((old_tid, col[row * sz..(row + 1) * sz].to_vec()));
                if self.archetypes[arch_id]
                    .changed
                    .get(&old_tid)
                    .is_some_and(|s| s.contains(&entity))
                {
                    preserved_changed.push(old_tid);
                }
            }
            copied.push((tid, slice.to_vec()));

            let moved = self.archetypes[arch_id].swap_remove_row(row);
            if let Some(moved_ent) = moved {
                self.entity_loc.insert(moved_ent, (arch_id, row));
            }

            let refs: Vec<(i64, &[u8])> = copied.iter().map(|(t, b)| (*t, b.as_slice())).collect();
            let new_row = self.archetypes[new_arch_id].push_row(entity, &refs);
            for old_tid in preserved_changed {
                if let Some(ch) = self.archetypes[new_arch_id].changed.get_mut(&old_tid) {
                    ch.insert(entity);
                }
            }
            if let Some(ch) = self.archetypes[new_arch_id].changed.get_mut(&tid) {
                ch.insert(entity);
            }
            self.entity_loc.insert(entity, (new_arch_id, new_row));
            self.added_events.entry(tid).or_default().insert(entity);
        } else {
            let mut sizes = HashMap::new();
            sizes.insert(tid, elem_size);
            let arch_id = self.get_or_create_archetype(vec![tid], sizes);
            let row = self.archetypes[arch_id].push_row(entity, &[(tid, slice)]);
            if let Some(ch) = self.archetypes[arch_id].changed.get_mut(&tid) {
                ch.insert(entity);
            }
            self.entity_loc.insert(entity, (arch_id, row));
            self.added_events.entry(tid).or_default().insert(entity);
        }
    }

    fn remove_dense(&mut self, entity: i64, tid: i64) -> bool {
        let Some(&(arch_id, row)) = self.entity_loc.get(&entity) else {
            return false;
        };
        if !self.archetypes[arch_id].contains(tid) {
            return false;
        }
        let mut new_sig = self.archetypes[arch_id].signature.clone();
        if let Ok(pos) = new_sig.binary_search(&tid) {
            new_sig.remove(pos);
        }

        if new_sig.is_empty() {
            let moved = self.archetypes[arch_id].swap_remove_row(row);
            if let Some(moved_ent) = moved {
                self.entity_loc.insert(moved_ent, (arch_id, row));
            }
            self.entity_loc.remove(&entity);
        } else {
            let mut new_sizes = self.archetypes[arch_id].elem_sizes.clone();
            new_sizes.remove(&tid);
            let new_arch_id = self.get_or_create_archetype(new_sig.clone(), new_sizes);

            let mut copied: Vec<(i64, Vec<u8>)> = Vec::with_capacity(new_sig.len());
            for &keep_tid in &new_sig {
                let sz = self.archetypes[arch_id].elem_sizes[&keep_tid];
                let col = &self.archetypes[arch_id].columns[&keep_tid];
                copied.push((keep_tid, col[row * sz..(row + 1) * sz].to_vec()));
            }

            let moved = self.archetypes[arch_id].swap_remove_row(row);
            if let Some(moved_ent) = moved {
                self.entity_loc.insert(moved_ent, (arch_id, row));
            }

            let refs: Vec<(i64, &[u8])> = copied.iter().map(|(t, b)| (*t, b.as_slice())).collect();
            let new_row = self.archetypes[new_arch_id].push_row(entity, &refs);
            self.entity_loc.insert(entity, (new_arch_id, new_row));
        }

        self.removed_events.entry(tid).or_default().insert(entity);
        true
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn striv_ecs_new() -> *mut StrivEcs {
    Box::into_raw(Box::new(StrivEcs::new()))
}

#[unsafe(no_mangle)]
pub extern "C" fn striv_ecs_mark_sparse(ecs: *mut StrivEcs, type_id: i64) -> i64 {
    let w = unsafe { &mut *ecs };
    w.sparse_types.insert(type_id);
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn striv_ecs_add_rel(
    ecs: *mut StrivEcs,
    entity: i64,
    rel_tid: i64,
    target: i64,
) -> i64 {
    let w = unsafe { &mut *ecs };
    let targets = w.relations.entry((entity, rel_tid)).or_default();
    if !targets.contains(&target) {
        targets.push(target);
    }
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn striv_ecs_remove_rel(
    ecs: *mut StrivEcs,
    entity: i64,
    rel_tid: i64,
    target: i64,
) -> i64 {
    let w = unsafe { &mut *ecs };
    if let Some(targets) = w.relations.get_mut(&(entity, rel_tid)) {
        if let Some(pos) = targets.iter().position(|&t| t == target) {
            targets.swap_remove(pos);
            if targets.is_empty() {
                w.relations.remove(&(entity, rel_tid));
            }
            return 1;
        }
    }
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn striv_ecs_has_rel(
    ecs: *const StrivEcs,
    entity: i64,
    rel_tid: i64,
    target: i64,
) -> i64 {
    let w = unsafe { &*ecs };
    w.relations
        .get(&(entity, rel_tid))
        .is_some_and(|targets| targets.contains(&target)) as i64
}

#[unsafe(no_mangle)]
pub extern "C" fn striv_ecs_rel_target(ecs: *const StrivEcs, entity: i64, rel_tid: i64) -> i64 {
    let w = unsafe { &*ecs };
    w.relations
        .get(&(entity, rel_tid))
        .and_then(|targets| targets.first().copied())
        .unwrap_or(0)
}

#[unsafe(no_mangle)]
pub extern "C" fn striv_ecs_was_added(ecs: *const StrivEcs, entity: i64, type_id: i64) -> i64 {
    let w = unsafe { &*ecs };
    w.added_events
        .get(&type_id)
        .is_some_and(|s| s.contains(&entity)) as i64
}

#[unsafe(no_mangle)]
pub extern "C" fn striv_ecs_was_removed(ecs: *const StrivEcs, entity: i64, type_id: i64) -> i64 {
    let w = unsafe { &*ecs };
    w.removed_events
        .get(&type_id)
        .is_some_and(|s| s.contains(&entity)) as i64
}

#[unsafe(no_mangle)]
pub extern "C" fn striv_ecs_clear_events(ecs: *mut StrivEcs) -> i64 {
    let w = unsafe { &mut *ecs };
    w.added_events.clear();
    w.removed_events.clear();
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn striv_ecs_archetype_count(ecs: *const StrivEcs) -> i64 {
    let w = unsafe { &*ecs };
    w.archetypes.len() as i64
}

#[unsafe(no_mangle)]
pub extern "C" fn striv_ecs_entity_count(ecs: *const StrivEcs) -> i64 {
    let w = unsafe { &*ecs };
    let mut all: HashSet<i64> = w.entity_loc.keys().copied().collect();
    for pool in w.sparse_pools.values() {
        for &e in &pool.entities {
            all.insert(e);
        }
    }
    all.len() as i64
}

#[unsafe(no_mangle)]
pub extern "C" fn striv_ecs_set_group(ecs: *mut StrivEcs, entity: i64, group_id: i64) -> i64 {
    let w = unsafe { &mut *ecs };
    w.groups.insert(entity, group_id);
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn striv_ecs_group_of(ecs: *const StrivEcs, entity: i64) -> i64 {
    let w = unsafe { &*ecs };
    w.groups.get(&entity).copied().unwrap_or(0)
}

#[unsafe(no_mangle)]
pub extern "C" fn striv_ecs_in_same_group(ecs: *const StrivEcs, a: i64, b: i64) -> i64 {
    let w = unsafe { &*ecs };
    (w.groups.get(&a).copied().unwrap_or(0) == w.groups.get(&b).copied().unwrap_or(0)) as i64
}

#[unsafe(no_mangle)]
pub extern "C" fn striv_ecs_is_changed(ecs: *const StrivEcs, entity: i64, type_id: i64) -> i64 {
    let w = unsafe { &*ecs };
    if w.sparse_types.contains(&type_id) {
        return w
            .sparse_pools
            .get(&type_id)
            .is_some_and(|p| p.changed.contains(&entity)) as i64;
    }
    if let Some(&(arch_id, _)) = w.entity_loc.get(&entity) {
        return w.archetypes[arch_id]
            .changed
            .get(&type_id)
            .is_some_and(|s| s.contains(&entity)) as i64;
    }
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn striv_ecs_clear_changed(ecs: *mut StrivEcs) -> i64 {
    let w = unsafe { &mut *ecs };
    for pool in w.sparse_pools.values_mut() {
        pool.changed.clear();
    }
    for arch in &mut w.archetypes {
        for set in arch.changed.values_mut() {
            set.clear();
        }
    }
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn striv_ecs_set_parent(ecs: *mut StrivEcs, child: i64, parent: i64) -> i64 {
    let w = unsafe { &mut *ecs };
    if child != parent && parent > 0 {
        w.parents.insert(child, parent);
    }
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn striv_ecs_detach(ecs: *mut StrivEcs, child: i64) -> i64 {
    let w = unsafe { &mut *ecs };
    w.parents.remove(&child);
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn striv_ecs_parent_of(ecs: *const StrivEcs, child: i64) -> i64 {
    let w = unsafe { &*ecs };
    w.parents.get(&child).copied().unwrap_or(0)
}

#[unsafe(no_mangle)]
pub extern "C" fn striv_ecs_has(ecs: *const StrivEcs, entity: i64, type_id: i64) -> i64 {
    let w = unsafe { &*ecs };
    if w.sparse_types.contains(&type_id) {
        return w
            .sparse_pools
            .get(&type_id)
            .is_some_and(|p| p.index_of.contains_key(&entity)) as i64;
    }
    w.entity_loc
        .get(&entity)
        .is_some_and(|&(arch_id, _)| w.archetypes[arch_id].contains(type_id)) as i64
}

#[unsafe(no_mangle)]
pub extern "C" fn striv_ecs_remove_comp(ecs: *mut StrivEcs, entity: i64, type_id: i64) -> i64 {
    let w = unsafe { &mut *ecs };
    if w.sparse_types.contains(&type_id) {
        if let Some(pool) = w.sparse_pools.get_mut(&type_id) {
            if pool.remove(entity) {
                w.removed_events.entry(type_id).or_default().insert(entity);
                return 1;
            }
        }
        return 0;
    }
    w.remove_dense(entity, type_id) as i64
}

#[unsafe(no_mangle)]
pub extern "C" fn striv_ecs_spawn(ecs: *mut StrivEcs) -> i64 {
    let w = unsafe { &mut *ecs };
    let id = w.next_entity;
    w.next_entity += 1;
    id
}

#[unsafe(no_mangle)]
pub extern "C" fn striv_ecs_set(
    ecs: *mut StrivEcs,
    entity: i64,
    type_id: i64,
    elem_size: i64,
    src_ptr: *const u8,
) -> i64 {
    let w = unsafe { &mut *ecs };
    let sz = (elem_size as usize).max(1);
    let slice = unsafe { std::slice::from_raw_parts(src_ptr, sz) };
    if w.sparse_types.contains(&type_id) {
        let pool = w
            .sparse_pools
            .entry(type_id)
            .or_insert_with(|| ComponentPool::new(sz));
        if pool.upsert(entity, slice) {
            w.added_events.entry(type_id).or_default().insert(entity);
        }
        return 0;
    }
    w.set_dense(entity, type_id, sz, slice);
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn striv_ecs_get_ptr(ecs: *mut StrivEcs, entity: i64, type_id: i64) -> *mut u8 {
    let w = unsafe { &mut *ecs };
    if w.sparse_types.contains(&type_id) {
        return w
            .sparse_pools
            .get_mut(&type_id)
            .and_then(|p| p.get_ptr(entity))
            .unwrap_or(std::ptr::null_mut());
    }
    if let Some(&(arch_id, row)) = w.entity_loc.get(&entity) {
        return w.archetypes[arch_id]
            .get_ptr(row, type_id)
            .unwrap_or(std::ptr::null_mut());
    }
    std::ptr::null_mut()
}

#[unsafe(no_mangle)]
pub extern "C" fn striv_ecs_remove_entity(ecs: *mut StrivEcs, root_entity: i64) -> i64 {
    let w = unsafe { &mut *ecs };
    let mut to_delete = vec![root_entity];
    let mut i = 0;
    while i < to_delete.len() {
        let current = to_delete[i];
        for (&child, &parent) in &w.parents {
            if parent == current && !to_delete.contains(&child) {
                to_delete.push(child);
            }
        }
        i += 1;
    }

    for ent in to_delete {
        w.parents.remove(&ent);
        w.groups.remove(&ent);
        w.relations
            .retain(|&(src, _), targets| {
                if src == ent {
                    return false;
                }
                targets.retain(|&t| t != ent);
                !targets.is_empty()
            });

        if let Some((arch_id, row)) = w.entity_loc.remove(&ent) {
            let sig = w.archetypes[arch_id].signature.clone();
            for tid in sig {
                w.removed_events.entry(tid).or_default().insert(ent);
            }
            if let Some(moved_ent) = w.archetypes[arch_id].swap_remove_row(row) {
                w.entity_loc.insert(moved_ent, (arch_id, row));
            }
        }

        for (&tid, pool) in w.sparse_pools.iter_mut() {
            if pool.remove(ent) {
                w.removed_events.entry(tid).or_default().insert(ent);
            }
        }
    }
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn striv_ecs_query_run(
    ecs: *mut StrivEcs,
    type_ids_ptr: *const i64,
    comp_count: i64,
    body_fn_addr: usize,
    caps_ptr: *const i64,
    cap_count: i64,
    is_parallel: i64,
    exclude_tids_ptr: *const i64,
    exclude_count: i64,
    pass_entity_id: i64,
) {
    let w = unsafe { &mut *ecs };
    let n_comps = comp_count as usize;
    if n_comps == 0 {
        return;
    }
    let tids = unsafe { std::slice::from_raw_parts(type_ids_ptr, n_comps) };
    let caps = if cap_count > 0 {
        unsafe { std::slice::from_raw_parts(caps_ptr, cap_count as usize) }
    } else {
        &[]
    };
    let excl = if exclude_count > 0 {
        unsafe { std::slice::from_raw_parts(exclude_tids_ptr, exclude_count as usize) }
    } else {
        &[]
    };

    let mut dense_req: Vec<i64> = Vec::new();
    let mut sparse_req: Vec<i64> = Vec::new();
    for &tid in tids {
        if w.sparse_types.contains(&tid) {
            sparse_req.push(tid);
        } else {
            dense_req.push(tid);
        }
    }
    let mut dense_excl: Vec<i64> = Vec::new();
    let mut sparse_excl: Vec<i64> = Vec::new();
    for &tid in excl {
        if w.sparse_types.contains(&tid) {
            sparse_excl.push(tid);
        } else {
            dense_excl.push(tid);
        }
    }

    let stride = n_comps + if pass_entity_id != 0 { 1 } else { 0 };
    let mut matched_ptrs: Vec<usize> = Vec::new();

    if dense_req.is_empty() {
        // Pure sparse-set query: drive from smallest sparse pool
        let mut driver_tid = sparse_req[0];
        let mut min_len = usize::MAX;
        for &tid in &sparse_req {
            match w.sparse_pools.get(&tid) {
                Some(p) if p.entities.len() < min_len => {
                    min_len = p.entities.len();
                    driver_tid = tid;
                }
                None => return,
                _ => {}
            }
        }
        let candidates = w.sparse_pools[&driver_tid].entities.clone();
        matched_ptrs.reserve(candidates.len() * stride);
        'ent_loop: for ent in candidates {
            for &ex_tid in &sparse_excl {
                if w.sparse_pools
                    .get(&ex_tid)
                    .is_some_and(|p| p.index_of.contains_key(&ent))
                {
                    continue 'ent_loop;
                }
            }
            if !dense_excl.is_empty() {
                if let Some(&(arch_id, _)) = w.entity_loc.get(&ent) {
                    for &ex_tid in &dense_excl {
                        if w.archetypes[arch_id].contains(ex_tid) {
                            continue 'ent_loop;
                        }
                    }
                }
            }
            let start = matched_ptrs.len();
            if pass_entity_id != 0 {
                matched_ptrs.push(ent as usize);
            }
            for &tid in tids {
                if let Some(ptr) = w.sparse_pools.get_mut(&tid).and_then(|p| p.get_ptr(ent)) {
                    matched_ptrs.push(ptr as usize);
                } else {
                    matched_ptrs.truncate(start);
                    continue 'ent_loop;
                }
            }
        }
    } else {
        // Hybrid Archetype + Sparse-Set query with Query Cache
        let mut cache_req = dense_req.clone();
        cache_req.sort_unstable();
        let mut cache_excl = dense_excl.clone();
        cache_excl.sort_unstable();
        let cache_key = (cache_req, cache_excl);

        let arch_ids = if let Some(cached) = w.query_cache.get(&cache_key) {
            cached.clone()
        } else {
            let mut matched = Vec::new();
            for (aid, arch) in w.archetypes.iter().enumerate() {
                if aid == 0 {
                    continue;
                }
                if cache_key.0.iter().all(|t| arch.contains(*t))
                    && cache_key.1.iter().all(|t| !arch.contains(*t))
                {
                    matched.push(aid);
                }
            }
            w.query_cache.insert(cache_key, matched.clone());
            matched
        };

        for aid in arch_ids {
            let arch = &mut w.archetypes[aid];
            let n_rows = arch.entities.len();
            if n_rows == 0 {
                continue;
            }
            matched_ptrs.reserve(n_rows * stride);

            // Pre-resolve dense column base pointers & sizes once per archetype
            let mut col_bases: Vec<(bool, *mut u8, usize, i64)> = Vec::with_capacity(n_comps);
            for &tid in tids {
                if w.sparse_types.contains(&tid) {
                    col_bases.push((true, std::ptr::null_mut(), 0, tid));
                } else {
                    let sz = arch.elem_sizes[&tid];
                    let base = arch.columns.get_mut(&tid).unwrap().as_mut_ptr();
                    col_bases.push((false, base, sz, tid));
                }
            }

            'row_loop: for row in 0..n_rows {
                let ent = arch.entities[row];
                for &ex_tid in &sparse_excl {
                    if w.sparse_pools
                        .get(&ex_tid)
                        .is_some_and(|p| p.index_of.contains_key(&ent))
                    {
                        continue 'row_loop;
                    }
                }
                let start = matched_ptrs.len();
                if pass_entity_id != 0 {
                    matched_ptrs.push(ent as usize);
                }
                for &(is_sparse, base, sz, tid) in &col_bases {
                    if is_sparse {
                        if let Some(ptr) = w.sparse_pools.get_mut(&tid).and_then(|p| p.get_ptr(ent))
                        {
                            matched_ptrs.push(ptr as usize);
                        } else {
                            matched_ptrs.truncate(start);
                            continue 'row_loop;
                        }
                    } else {
                        matched_ptrs.push(unsafe { base.add(row * sz) } as usize);
                    }
                }
            }
        }
    }

    if is_parallel != 0 {
        matched_ptrs.par_chunks_exact(stride).for_each(|row| unsafe {
            invoke_query_fn(body_fn_addr, row, caps);
        });
    } else {
        for row in matched_ptrs.chunks_exact(stride) {
            unsafe {
                invoke_query_fn(body_fn_addr, row, caps);
            }
        }
    }
}

unsafe fn invoke_query_fn(addr: usize, comps: &[usize], caps: &[i64]) {
    let total = comps.len() + caps.len();
    let mut all_args = [0i64; 12];
    assert!(
        total <= 12,
        "ECS query unterstützt bis zu 12 kombinierte Component- + Capture-Parameter"
    );
    for (i, &p) in comps.iter().enumerate() {
        all_args[i] = p as i64;
    }
    for (i, &c) in caps.iter().enumerate() {
        all_args[comps.len() + i] = c;
    }

    let ptr = addr as *const ();
    unsafe {
        match total {
            1 => {
                std::mem::transmute::<*const (), fn(i64) -> i64>(ptr)(all_args[0]);
            }
            2 => {
                std::mem::transmute::<*const (), fn(i64, i64) -> i64>(ptr)(
                    all_args[0],
                    all_args[1],
                );
            }
            3 => {
                std::mem::transmute::<*const (), fn(i64, i64, i64) -> i64>(ptr)(
                    all_args[0],
                    all_args[1],
                    all_args[2],
                );
            }
            4 => {
                std::mem::transmute::<*const (), fn(i64, i64, i64, i64) -> i64>(ptr)(
                    all_args[0],
                    all_args[1],
                    all_args[2],
                    all_args[3],
                );
            }
            5 => {
                std::mem::transmute::<*const (), fn(i64, i64, i64, i64, i64) -> i64>(ptr)(
                    all_args[0],
                    all_args[1],
                    all_args[2],
                    all_args[3],
                    all_args[4],
                );
            }
            6 => {
                std::mem::transmute::<*const (), fn(i64, i64, i64, i64, i64, i64) -> i64>(ptr)(
                    all_args[0],
                    all_args[1],
                    all_args[2],
                    all_args[3],
                    all_args[4],
                    all_args[5],
                );
            }
            7 => {
                std::mem::transmute::<*const (), fn(i64, i64, i64, i64, i64, i64, i64) -> i64>(ptr)(
                    all_args[0],
                    all_args[1],
                    all_args[2],
                    all_args[3],
                    all_args[4],
                    all_args[5],
                    all_args[6],
                );
            }
            8 => {
                std::mem::transmute::<*const (), fn(i64, i64, i64, i64, i64, i64, i64, i64) -> i64>(
                    ptr,
                )(
                    all_args[0],
                    all_args[1],
                    all_args[2],
                    all_args[3],
                    all_args[4],
                    all_args[5],
                    all_args[6],
                    all_args[7],
                );
            }
            9 => {
                std::mem::transmute::<
                    *const (),
                    fn(i64, i64, i64, i64, i64, i64, i64, i64, i64) -> i64,
                >(ptr)(
                    all_args[0],
                    all_args[1],
                    all_args[2],
                    all_args[3],
                    all_args[4],
                    all_args[5],
                    all_args[6],
                    all_args[7],
                    all_args[8],
                );
            }
            10 => {
                std::mem::transmute::<
                    *const (),
                    fn(i64, i64, i64, i64, i64, i64, i64, i64, i64, i64) -> i64,
                >(ptr)(
                    all_args[0],
                    all_args[1],
                    all_args[2],
                    all_args[3],
                    all_args[4],
                    all_args[5],
                    all_args[6],
                    all_args[7],
                    all_args[8],
                    all_args[9],
                );
            }
            11 => {
                std::mem::transmute::<
                    *const (),
                    fn(i64, i64, i64, i64, i64, i64, i64, i64, i64, i64, i64) -> i64,
                >(ptr)(
                    all_args[0],
                    all_args[1],
                    all_args[2],
                    all_args[3],
                    all_args[4],
                    all_args[5],
                    all_args[6],
                    all_args[7],
                    all_args[8],
                    all_args[9],
                    all_args[10],
                );
            }
            12 => {
                std::mem::transmute::<
                    *const (),
                    fn(i64, i64, i64, i64, i64, i64, i64, i64, i64, i64, i64, i64) -> i64,
                >(ptr)(
                    all_args[0],
                    all_args[1],
                    all_args[2],
                    all_args[3],
                    all_args[4],
                    all_args[5],
                    all_args[6],
                    all_args[7],
                    all_args[8],
                    all_args[9],
                    all_args[10],
                    all_args[11],
                );
            }
            _ => {}
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn striv_ecs_free(ecs: *mut StrivEcs) {
    if !ecs.is_null() {
        unsafe {
            drop(Box::from_raw(ecs));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_archetype_migration_and_sparse_hybrid() {
        let ecs = striv_ecs_new();
        let e1 = striv_ecs_spawn(ecs);
        let e2 = striv_ecs_spawn(ecs);

        let tid_pos = 100;
        let tid_vel = 200;
        let tid_tag = 300;
        striv_ecs_mark_sparse(ecs, tid_tag);

        let pos1: i64 = 42;
        let vel1: i64 = 99;
        let tag1: i64 = 1;

        striv_ecs_set(ecs, e1, tid_pos, 8, &pos1 as *const i64 as *const u8);
        striv_ecs_set(ecs, e1, tid_vel, 8, &vel1 as *const i64 as *const u8);
        striv_ecs_set(ecs, e2, tid_pos, 8, &pos1 as *const i64 as *const u8);
        // Adding sparse component should NOT create a new archetype
        let arch_before = striv_ecs_archetype_count(ecs);
        striv_ecs_set(ecs, e1, tid_tag, 8, &tag1 as *const i64 as *const u8);
        assert_eq!(striv_ecs_archetype_count(ecs), arch_before);

        assert_eq!(striv_ecs_has(ecs, e1, tid_pos), 1);
        assert_eq!(striv_ecs_has(ecs, e1, tid_tag), 1);
        assert_eq!(striv_ecs_entity_count(ecs), 2);
        assert_eq!(striv_ecs_was_added(ecs, e1, tid_pos), 1);
        assert_eq!(striv_ecs_is_changed(ecs, e1, tid_pos), 1);

        striv_ecs_remove_comp(ecs, e1, tid_vel);
        assert_eq!(striv_ecs_has(ecs, e1, tid_vel), 0);
        assert_eq!(striv_ecs_was_removed(ecs, e1, tid_vel), 1);

        striv_ecs_clear_events(ecs);
        assert_eq!(striv_ecs_was_added(ecs, e1, tid_pos), 0);
        assert_eq!(striv_ecs_was_removed(ecs, e1, tid_vel), 0);

        striv_ecs_free(ecs);
    }

    #[test]
    fn test_relationships_and_cleanup() {
        let ecs = striv_ecs_new();
        let ship = striv_ecs_spawn(ecs);
        let kerbin = striv_ecs_spawn(ecs);
        let rel_orbit = 777;

        striv_ecs_add_rel(ecs, ship, rel_orbit, kerbin);
        assert_eq!(striv_ecs_has_rel(ecs, ship, rel_orbit, kerbin), 1);
        assert_eq!(striv_ecs_rel_target(ecs, ship, rel_orbit), kerbin);

        striv_ecs_remove_entity(ecs, kerbin);
        assert_eq!(striv_ecs_has_rel(ecs, ship, rel_orbit, kerbin), 0);
        assert_eq!(striv_ecs_rel_target(ecs, ship, rel_orbit), 0);

        striv_ecs_free(ecs);
    }
}
