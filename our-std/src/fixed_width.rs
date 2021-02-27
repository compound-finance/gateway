/// Fixed symbol width.
pub const WIDTH: usize = 12;

pub const fn str_to_label(s: &str) -> [u8; WIDTH] {
    assert!(s.len() < WIDTH, "Too many chars");
    let mut bytes = [0u8; WIDTH];
    let mut i = 0;
    let raw = s.as_bytes();
    loop {
        if i >= s.len() {
            break;
        }
        bytes[i] = raw[i];
        i += 1;
    }
    bytes
}

pub fn label_to_string(l: [u8; WIDTH]) -> String {
    let mut s = String::with_capacity(WIDTH);
    let bytes = &l[..];
    for i in 0..WIDTH {
        if bytes[i] == 0 {
            break;
        }
        s.push(bytes[i] as char);
    }
    s
}
