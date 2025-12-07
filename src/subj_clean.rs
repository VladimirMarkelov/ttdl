use crate::fmt::Conf;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Hide {
    Nothing,
    Tags,
    All,
}

pub fn hide_contexts(subj: &mut String, c: &Conf) {
    if c.hide != Hide::All {
        return;
    }
    hide_any(subj, " @")
}

pub fn hide_projects(subj: &mut String, c: &Conf) {
    if c.hide != Hide::All {
        return;
    }
    hide_any(subj, " +")
}

pub fn hide_tags(subj: &mut String, tag: &str, c: &Conf) {
    if c.hide == Hide::Nothing {
        return;
    }
    if tag.is_empty() {
        return;
    }
    let tg = if tag.ends_with(':') { format!(" {tag}") } else { format!(" {tag}:") };
    hide_any(subj, &tg)
}

pub fn hide_any(subj: &mut String, what: &str) {
    let mut st = format!(" {subj} ");
    let mut s = st.as_str();
    if !s.contains(what) {
        return;
    }
    let mut ii = 0;
    loop {
        ii += 1;
        if ii > 10 {
            return;
        }
        let idx = s.find(what);
        match idx {
            None => {
                *subj = s.trim().to_string();
                return;
            }
            Some(pos) => match &s[pos + 1..].find(' ') {
                None => s = &s[..pos + 1],
                Some(p) => {
                    st = s[..pos + 1].to_string() + &s[pos + 1 + p + 1..];
                    s = st.as_str();
                }
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hide_project_test() {
        let c = Conf { hide: Hide::All, ..Conf::default() };
        struct Test {
            s: &'static str,
            v: &'static str,
        }
        let tests: Vec<Test> = vec![
            Test { s: "+proj test", v: "test" },
            Test { s: "test +proj passed", v: "test passed" },
            Test { s: "test +proj", v: "test" },
            Test { s: "test +proj", v: "test" },
            Test { s: "test +proj +another something", v: "test something" },
            Test { s: "test one +proj two +another something", v: "test one two something" },
        ];
        for test in tests.iter() {
            let mut s = test.s.to_string();
            hide_projects(&mut s, &c);
            assert_eq!(s, test.v, "\n{}: {} != {}", test.s, test.v, s);
        }
    }

    #[test]
    fn hide_context_test() {
        let c = Conf { hide: Hide::All, ..Conf::default() };
        struct Test {
            s: &'static str,
            v: &'static str,
        }
        let tests: Vec<Test> = vec![
            Test { s: "@proj test", v: "test" },
            Test { s: "test @proj passed", v: "test passed" },
            Test { s: "test @proj", v: "test" },
            Test { s: "test @proj", v: "test" },
            Test { s: "test @proj @another something", v: "test something" },
            Test { s: "test one @proj two @another something", v: "test one two something" },
        ];
        for test in tests.iter() {
            let mut s = test.s.to_string();
            hide_contexts(&mut s, &c);
            assert_eq!(s, test.v, "\n{}: {} != {}", test.s, test.v, s);
        }
    }

    #[test]
    fn hide_tags_test() {
        let c = Conf { hide: Hide::Tags, ..Conf::default() };
        struct Test {
            s: &'static str,
            v: &'static str,
            t: &'static str,
        }
        let tests: Vec<Test> = vec![
            Test { s: "due:23 test", v: "test", t: "due" },
            Test { s: "due:23 test", v: "test", t: "due:" },
            Test { s: "due:23 test", v: "due:23 test", t: "du" },
            Test { s: "test due:23", v: "test", t: "due" },
            Test { s: "test due:23 middle", v: "test middle", t: "due" },
            Test { s: "test due:23 due:484848 multi due:abcd", v: "test multi", t: "due" },
            Test { s: "test due:23 one:484848 multi due:abcd", v: "test one:484848 multi", t: "due" },
            Test { s: "test due:23 one:484848 multi due:abcd", v: "test due:23 multi due:abcd", t: "one" },
        ];
        for test in tests.iter() {
            let mut s = test.s.to_string();
            hide_tags(&mut s, test.t, &c);
            assert_eq!(s, test.v, "\n{}: {} != {s}", test.s, test.v);
        }
    }

    #[test]
    fn hide_nothing_test() {
        struct Test {
            s: &'static str,
            v: &'static str,
            h: Hide,
        }
        let tests: Vec<Test> = vec![
            Test { s: "+proj test @ctx", v: "+proj test @ctx", h: Hide::Nothing },
            Test { s: "+proj test @ctx", v: "+proj test @ctx", h: Hide::Tags },
        ];
        for test in tests.iter() {
            let c = Conf { hide: test.h, ..Conf::default() };
            let mut s = test.s.to_string();
            hide_projects(&mut s, &c);
            assert_eq!(s, test.v, "\n{}: {} != {}", test.s, test.v, s);
        }
    }
}
