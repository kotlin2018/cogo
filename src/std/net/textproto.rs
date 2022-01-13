// TrimString returns s without leading and trailing ASCII space.
pub fn trim_string(b: &str) -> String {
    let mut b = b.to_string().into_bytes();
    while b.len() > 0 && is_asciispace(&b[0]) {
        b.remove(0);
    }
    while b.len() > 0 && is_asciispace(b.last().unwrap()) {
        b.pop();
    }
    return String::from_utf8(b).unwrap_or_default();
}

fn is_asciispace(b: &u8) -> bool {
    *b == ' ' as u8 || *b == '\t' as u8 || *b == '\n' as u8 || *b == '\r' as u8
}
