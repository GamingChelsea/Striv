pub fn random_int(min: i64, max: i64) -> i64 {
    min + (max - min) / 2
}

pub fn clamp_val(val: i64, min: i64, max: i64) -> i64 {
    val.clamp(min, max)
}
