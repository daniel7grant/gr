pub fn to_fixed_length(s: &str, len: usize, ellipsis: bool) -> String {
    let max_len = if ellipsis { len - 3 } else { len };
    if s.len() > max_len {
        format!("{}{}", &s[0..max_len], if ellipsis { "..." } else { "" })
    } else {
        format!("{:<width$}", s, width = len)
    }
}
