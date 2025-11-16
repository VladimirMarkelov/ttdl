use unicode_width::UnicodeWidthStr;

const KB: u64 = 1024;
const MB: u64 = KB * 1024;
const GB: u64 = MB * 1024;
const TB: u64 = GB * 1024;
const PB: u64 = TB * 1024;
const EB: u64 = PB * 1024;

const SEC_IN_MINUTE: i64 = 60;
const SEC_IN_HOUR: i64 = SEC_IN_MINUTE * 60;
const SEC_IN_DAY: i64 = SEC_IN_HOUR * 24;
const SEC_IN_WEEK: i64 = SEC_IN_DAY * 7;

struct ByteSize {
    s: &'static str,
    m: u64,
}

lazy_static! {
    static ref BYTES: [ByteSize; 18] = [
        ByteSize { s: "eib", m: EB },
        ByteSize { s: "eb", m: EB },
        ByteSize { s: "e", m: EB },
        ByteSize { s: "pib", m: PB },
        ByteSize { s: "pb", m: PB },
        ByteSize { s: "p", m: PB },
        ByteSize { s: "tib", m: TB },
        ByteSize { s: "tb", m: TB },
        ByteSize { s: "t", m: TB },
        ByteSize { s: "gib", m: GB },
        ByteSize { s: "gb", m: GB },
        ByteSize { s: "g", m: GB },
        ByteSize { s: "mib", m: MB },
        ByteSize { s: "mb", m: MB },
        ByteSize { s: "m", m: MB },
        ByteSize { s: "kib", m: KB },
        ByteSize { s: "kb", m: KB },
        ByteSize { s: "k", m: KB },
    ];
}

pub fn cut_string(s: &str, max_width: usize) -> &str {
    let w = s.width();
    if max_width == 0 || w <= max_width {
        return s;
    }
    match s.char_indices().nth(max_width) {
        Some((pos, _)) => &s[..pos],
        None => s,
    }
}
pub fn str_to_bytes(s: &str) -> Option<u64> {
    let l = s.to_lowercase();
    for sz in BYTES.iter() {
        if l.ends_with(sz.s) {
            let lv = l.trim_end_matches(sz.s);
            let sb = lv.parse::<u64>().ok()?;
            return Some(sb * sz.m);
        }
    }
    s.parse::<u64>().ok()
}
pub fn str_to_duration(s: &str) -> Option<i64> {
    let l = s.to_lowercase();
    let s = l.as_str();
    let mut dur: i64 = 0;
    let (sgn, mut s) = if s.starts_with('-') { (-1i64, s.trim_start_matches('-')) } else { (1i64, s) };
    loop {
        if s.is_empty() {
            return Some(dur * sgn);
        }
        match s.find(|c: char| !c.is_ascii_digit()) {
            None => {
                let v = s.parse::<u32>().ok()?;
                dur = (dur + v as i64) * sgn;
                return Some(dur);
            }
            Some(pos) => {
                let vs = &s[..pos];
                let value = vs.parse::<u32>().ok()?;
                s = &s[pos..];
                let suffix = match s.find(|c: char| c.is_ascii_digit()) {
                    None => {
                        let save = s;
                        s = &s[..0];
                        save
                    }
                    Some(pos) => {
                        let save = &s[..pos];
                        s = &s[pos..];
                        save
                    }
                };
                let value: i64 = match suffix {
                    "w" => value as i64 * SEC_IN_WEEK,
                    "d" => value as i64 * SEC_IN_DAY,
                    "h" => value as i64 * SEC_IN_HOUR,
                    "m" => value as i64 * SEC_IN_MINUTE,
                    "s" | "" => value as i64,
                    _ => return None,
                };
                dur += value;
            }
        }
    }
}
pub fn str_to_time(s: &str) -> Option<u32> {
    let s = s.to_lowercase();
    match s.find(|c: char| !c.is_ascii_digit()) {
        None => {
            if s.len() < 3 {
                return None;
            }
            let v = s.parse::<u32>().ok()?;
            let h = v / 100;
            let m = v % 100;
            if h > 23 || m > 59 {
                return None;
            }
            Some(v)
        }
        Some(pos) => {
            let vs = &s[..pos];
            let mut v = vs.parse::<u32>().ok()?;
            let suffix = &s[pos..];
            if suffix != "am" && suffix != "pm" {
                return None;
            }
            let h = v / 100;
            let m = v % 100;
            if h > 12 || h == 0 || m > 59 {
                return None;
            }
            if (1200..=1259).contains(&v) && suffix == "am" {
                v -= 1200;
            }
            if v < 1200 && suffix == "pm" {
                v += 1200;
            }
            Some(v)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bytes_test() {
        struct Test {
            s: &'static str,
            v: u64,
        }
        let tests: Vec<Test> = vec![
            Test { s: "4789", v: 4789 },
            Test { s: "5k", v: 5 * KB },
            Test { s: "2MiB", v: 2 * MB },
            Test { s: "188Gb", v: 188 * GB },
            Test { s: "24P", v: 24 * PB },
        ];
        for test in tests.iter() {
            let v = str_to_bytes(test.s).unwrap();
            assert_eq!(v, test.v, "\n{}: {} != {}", test.s, test.v, v);
        }
    }

    #[test]
    fn duration_test() {
        struct Test {
            s: &'static str,
            d: i64,
        }
        let tests: Vec<Test> = vec![
            Test { s: "7829", d: 7829 },
            Test { s: "1d89", d: SEC_IN_DAY + 89 },
            Test { s: "21h44m", d: SEC_IN_MINUTE * 44 + SEC_IN_HOUR * 21 },
            Test { s: "3w2s", d: SEC_IN_WEEK * 3 + 2 },
            Test { s: "-1m5s", d: -SEC_IN_MINUTE - 5 },
            Test {
                s: "11d12w13m14h10s",
                d: SEC_IN_WEEK * 12 + SEC_IN_DAY * 11 + SEC_IN_HOUR * 14 + SEC_IN_MINUTE * 13 + 10,
            },
        ];
        for test in tests.iter() {
            let v = str_to_duration(test.s).unwrap();
            assert_eq!(v, test.d, "\n{}: {} != {}", test.s, test.d, v);
        }
    }
    #[test]
    fn str_time_test() {
        struct Test {
            s: &'static str,
            d: Option<u32>,
        }
        let tests: Vec<Test> = vec![
            Test { s: "7829", d: None },
            Test { s: "1060", d: None },
            Test { s: "60", d: None },
            Test { s: "60am", d: None },
            Test { s: "1320am", d: None },
            Test { s: "1011tm", d: None },
            Test { s: "1030", d: Some(1030) },
            Test { s: "2359", d: Some(2359) },
            Test { s: "1011am", d: Some(1011) },
            Test { s: "1011pm", d: Some(2211) },
        ];
        for test in tests.iter() {
            let v = str_to_time(test.s);
            assert_eq!(v, test.d, "\n{0:?}: {1:?} != {2:?}", test.s, test.d, v);
        }
    }
}
