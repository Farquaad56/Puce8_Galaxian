//! Analyse minimale des arguments : positionnels + `--clé valeur` + `--drapeau`.

use std::collections::HashMap;

pub struct Args {
    pub pos: Vec<String>,
    pub opts: HashMap<String, String>,
}

impl Args {
    /// `flags` = options sans valeur (ex. `["strict-time"]`).
    pub fn parse(raw: &[String], flags: &[&str]) -> Args {
        let mut pos = Vec::new();
        let mut opts = HashMap::new();
        let mut i = 0;
        while i < raw.len() {
            if let Some(name) = raw[i].strip_prefix("--") {
                if flags.contains(&name) {
                    opts.insert(name.to_string(), String::new());
                } else if i + 1 < raw.len() {
                    opts.insert(name.to_string(), raw[i + 1].clone());
                    i += 1;
                }
            } else {
                pos.push(raw[i].clone());
            }
            i += 1;
        }
        Args { pos, opts }
    }

    pub fn get(&self, key: &str) -> Option<&str> {
        self.opts.get(key).map(String::as_str)
    }

    pub fn has(&self, key: &str) -> bool {
        self.opts.contains_key(key)
    }

    pub fn num<T: std::str::FromStr>(&self, key: &str, default: T) -> T {
        self.get(key)
            .and_then(|v| v.parse().ok())
            .unwrap_or(default)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(s: &[&str]) -> Vec<String> {
        s.iter().map(|x| x.to_string()).collect()
    }

    #[test]
    fn parses_positional_options_and_flags() {
        let a = Args::parse(
            &v(&["rom.zip", "--frames", "10", "--strict", "x"]),
            &["strict"],
        );
        assert_eq!(a.pos, vec!["rom.zip", "x"]);
        assert_eq!(a.num("frames", 0u32), 10);
        assert!(a.has("strict"));
        assert_eq!(a.get("nope"), None);
        assert_eq!(a.num("nope", 7u32), 7);
    }
}
