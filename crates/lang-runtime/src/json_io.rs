#[unsafe(no_mangle)]
pub extern "C" fn striv_struct_to_json(
    struct_ptr: *const u8,
    schema_ptr: *const u8,
    schema_len: i64,
) -> *mut String {
    let schema = unsafe {
        std::str::from_utf8_unchecked(std::slice::from_raw_parts(
            schema_ptr,
            schema_len as usize,
        ))
    };
    let mut out = String::from("{");
    let mut first = true;
    for entry in schema.split(',') {
        if entry.is_empty() {
            continue;
        }
        let mut parts = entry.split(':');
        let name = parts.next().unwrap_or("");
        let ty = parts.next().unwrap_or("int");
        let off: usize = parts.next().and_then(|s| s.parse().ok()).unwrap_or(0);
        if !first {
            out.push(',');
        }
        first = false;
        out.push_str(&format!("\"{}\":", name));
        unsafe {
            let p = struct_ptr.add(off);
            match ty {
                "float" => out.push_str(&format!("{}", *(p as *const f64))),
                "f32" => out.push_str(&format!("{}", *(p as *const f32))),
                "u32" => out.push_str(&format!("{}", *(p as *const u32))),
                "i32" => out.push_str(&format!("{}", *(p as *const i32))),
                "u8" | "bool" => out.push_str(&format!("{}", *p)),
                "i8" => out.push_str(&format!("{}", *(p as *const i8))),
                "String" => {
                    let s_ptr = *(p as *const *const String);
                    if s_ptr.is_null() {
                        out.push_str("\"\"");
                    } else {
                        out.push_str(&format!("\"{}\"", (*s_ptr).replace('"', "\\\"")));
                    }
                }
                _ => out.push_str(&format!("{}", *(p as *const i64))),
            }
        }
    }
    out.push('}');
    Box::into_raw(Box::new(out))
}

#[unsafe(no_mangle)]
pub extern "C" fn striv_struct_from_json(
    struct_ptr: *mut u8,
    schema_ptr: *const u8,
    schema_len: i64,
    json_ptr: *const String,
) -> i64 {
    let schema = unsafe {
        std::str::from_utf8_unchecked(std::slice::from_raw_parts(
            schema_ptr,
            schema_len as usize,
        ))
    };
    let json = unsafe { &*json_ptr };
    for entry in schema.split(',') {
        if entry.is_empty() {
            continue;
        }
        let mut parts = entry.split(':');
        let name = parts.next().unwrap_or("");
        let ty = parts.next().unwrap_or("int");
        let off: usize = parts.next().and_then(|s| s.parse().ok()).unwrap_or(0);
        let key_pat = format!("\"{}\":", name);
        if let Some(pos) = json.find(&key_pat) {
            let rest = json[pos + key_pat.len()..].trim_start();
            unsafe {
                let p = struct_ptr.add(off);
                if ty == "String" {
                    if let Some(after_q) = rest.strip_prefix('"') {
                        if let Some(end_q) = after_q.find('"') {
                            let val = after_q[..end_q].to_string();
                            *(p as *mut *mut String) = Box::into_raw(Box::new(val));
                        }
                    }
                } else {
                    let end = rest.find([',', '}', ' ', '\n', '\r']).unwrap_or(rest.len());
                    let num_s = rest[..end].trim();
                    match ty {
                        "float" => {
                            if let Ok(v) = num_s.parse::<f64>() {
                                *(p as *mut f64) = v;
                            }
                        }
                        "f32" => {
                            if let Ok(v) = num_s.parse::<f32>() {
                                *(p as *mut f32) = v;
                            }
                        }
                        "u32" => {
                            if let Ok(v) = num_s.parse::<u32>() {
                                *(p as *mut u32) = v;
                            }
                        }
                        "i32" => {
                            if let Ok(v) = num_s.parse::<i32>() {
                                *(p as *mut i32) = v;
                            }
                        }
                        "u8" | "bool" => {
                            if let Ok(v) = num_s.parse::<u8>() {
                                *p = v;
                            }
                        }
                        "i8" => {
                            if let Ok(v) = num_s.parse::<i8>() {
                                *(p as *mut i8) = v;
                            }
                        }
                        _ => {
                            if let Ok(v) = num_s.parse::<i64>() {
                                *(p as *mut i64) = v;
                            }
                        }
                    }
                }
            }
        }
    }
    0
}
