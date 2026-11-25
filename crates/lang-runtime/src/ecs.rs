use rayon::prelude::*;
use std::collections::HashMap;

pub struct ComponentPool {
    pub elem_size: usize,
    pub entities: Vec<i64>,
    pub data: Vec<u8>,
    pub index_of: HashMap<i64, usize>,
}

impl ComponentPool {
    fn new(elem_size: usize) -> Self {
        Self {
            elem_size: elem_size.max(1),
            entities: Vec::new(),
            data: Vec::new(),
            index_of: HashMap::new(),
        }
    }

    fn upsert(&mut self, entity: i64, bytes: &[u8]) {
        if let Some(&idx) = self.index_of.get(&entity) {
            let off = idx * self.elem_size;
            self.data[off..off + self.elem_size].copy_from_slice(bytes);
        } else {
            let idx = self.entities.len();
            self.entities.push(entity);
            self.data.extend_from_slice(bytes);
            self.index_of.insert(entity, idx);
        }
    }

    #[inline]
    fn get_ptr(&mut self, entity: i64) -> Option<*mut u8> {
        let &idx = self.index_of.get(&entity)?;
        Some(unsafe { self.data.as_mut_ptr().add(idx * self.elem_size) })
    }
}

pub struct StrivEcs {
    next_entity: i64,
    pools: HashMap<i64, ComponentPool>,
}

#[unsafe(no_mangle)]
pub extern "C" fn striv_ecs_new() -> *mut StrivEcs {
    Box::into_raw(Box::new(StrivEcs {
        next_entity: 1,
        pools: HashMap::new(),
    }))
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
    let pool = w
        .pools
        .entry(type_id)
        .or_insert_with(|| ComponentPool::new(elem_size as usize));
    let slice = unsafe { std::slice::from_raw_parts(src_ptr, elem_size as usize) };
    pool.upsert(entity, slice);
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn striv_ecs_get_ptr(ecs: *mut StrivEcs, entity: i64, type_id: i64) -> *mut u8 {
    let w = unsafe { &mut *ecs };
    w.pools
        .get_mut(&type_id)
        .and_then(|p| p.get_ptr(entity))
        .unwrap_or(std::ptr::null_mut())
}

#[unsafe(no_mangle)]
pub extern "C" fn striv_ecs_remove_entity(ecs: *mut StrivEcs, entity: i64) -> i64 {
    let w = unsafe { &mut *ecs };
    for pool in w.pools.values_mut() {
        if let Some(idx) = pool.index_of.remove(&entity) {
            let last_idx = pool.entities.len() - 1;
            let sz = pool.elem_size;
            if idx != last_idx {
                let moved_ent = pool.entities[last_idx];
                pool.entities[idx] = moved_ent;
                pool.data
                    .copy_within(last_idx * sz..(last_idx + 1) * sz, idx * sz);
                pool.index_of.insert(moved_ent, idx);
            }
            pool.entities.pop();
            pool.data.truncate(last_idx * sz);
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

    let mut driver_tid = tids[0];
    let mut min_len = usize::MAX;
    for &tid in tids {
        match w.pools.get(&tid) {
            Some(p) if p.entities.len() < min_len => {
                min_len = p.entities.len();
                driver_tid = tid;
            }
            None => return,
            _ => {}
        }
    }

    let candidate_entities = w.pools[&driver_tid].entities.clone();

    let mut matched_rows: Vec<Vec<usize>> = Vec::with_capacity(candidate_entities.len());
    'ent_loop: for ent in candidate_entities {
        let mut row = Vec::with_capacity(n_comps);
        for &tid in tids {
            if let Some(ptr) = w.pools.get_mut(&tid).and_then(|p| p.get_ptr(ent)) {
                row.push(ptr as usize);
            } else {
                continue 'ent_loop;
            }
        }
        matched_rows.push(row);
    }

    if is_parallel != 0 {
        matched_rows.par_iter().for_each(|row| unsafe {
            invoke_query_fn(body_fn_addr, row, caps);
        });
    } else {
        for row in &matched_rows {
            unsafe {
                invoke_query_fn(body_fn_addr, row, caps);
            }
        }
    }
}

unsafe fn invoke_query_fn(addr: usize, comps: &[usize], caps: &[i64]) {
    let mut all_args = Vec::with_capacity(comps.len() + caps.len());
    for &p in comps {
        all_args.push(p as i64);
    }
    all_args.extend_from_slice(caps);

    let ptr = addr as *const ();
    unsafe {
        match all_args.len() {
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
            _ => panic!("ECS query unterstützt bis zu 6 kombinierte Component- + Capture-Parameter"),
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
