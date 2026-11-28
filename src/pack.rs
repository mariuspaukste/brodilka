//! World packs: a single text file that renames and re-flavours the world
//! without touching the engine. The engine ships only its own world; other
//! worlds are loaded at run time by the host (web/main.js) from a file the
//! player picks, so their content never has to live in this repository.
//!
//! The file is a small subset of TOML:
//!
//! ```toml
//! [pack]
//! id = "my-world"            # lowercase letters, digits and '-'
//! name = "My World"
//! subtitle = "A grim little realm"
//! seed = 1409                # optional: a fixed world seed
//!
//! [mon.wolf]                 # monsters by id: name, color, hp
//! name = "a warg"
//! color = "#4a4038"
//!
//! [item.katana]              # items by id: name, icon, plural
//! name = "long knife"
//!
//! [vendor.guard]             # vendors by id: title, names, chatter, robe
//! title = "the watchman"
//! names = ["Ned", "Tom"]
//! chatter = ["All quiet.", "Mind the road."]
//!
//! [spell.fireball]           # spells by id: name, words
//! [template.warrior]         # starting templates by id: name, desc
//!
//! [void.echoes]              # an isle of the Void: name, folk, count
//! folk = "orc"               # settles them there (`world/void.rs`)
//!
//! [text]                     # exact replacements of any shown text
//! "Graveyard" = "Barrow-downs"
//!
//! [words]                    # then replacements inside any shown text
//! "ankh shrine" = "waystone"
//! ```
//!
//! Strings use `"…"` with `\"`, `\\`, `\n`, `\t` and `\u{…}`/`\uXXXX` escapes;
//! arrays may span lines and end with a comma; `#` starts a comment.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Str(String),
    Int(i64),
    Bool(bool),
    List(Vec<Value>),
}

#[derive(Default, Debug)]
pub struct Pack {
    pub id: String,
    pub name: String,
    pub subtitle: String,
    pub seed: Option<u32>,
    /// `[section]` → key → value; `[pack]`, `[text]` and `[words]` included
    pub sections: HashMap<String, HashMap<String, Value>>,
    pub text: HashMap<String, String>,
    /// substring replacements, longest first so "ankh shrine" wins over "ankh"
    pub words: Vec<(String, String)>,
}

/// The engine's own world: no overrides at all.
pub fn builtin() -> Pack {
    Pack {
        id: "brodilka".into(),
        name: "Brodilka".into(),
        subtitle: "A realm of virtue and steel".into(),
        ..Pack::default()
    }
}

impl Pack {
    pub fn parse(src: &str) -> Result<Pack, String> {
        let sections = parse_toml(src)?;
        let meta = sections
            .get("pack")
            .ok_or("a pack needs a [pack] section with an id")?;
        let id = match meta.get("id") {
            Some(Value::Str(s)) => s.clone(),
            _ => return Err("[pack] needs id = \"…\"".into()),
        };
        if id.is_empty()
            || !id
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
        {
            return Err(format!(
                "pack id \"{id}\" may use only lowercase letters, digits and '-'"
            ));
        }
        let s = |k: &str| match meta.get(k) {
            Some(Value::Str(s)) => s.clone(),
            _ => String::new(),
        };
        let seed = match meta.get("seed") {
            None => None,
            Some(Value::Int(n)) if (0..=u32::MAX as i64).contains(n) => Some(*n as u32),
            Some(_) => return Err("[pack] seed must be a whole number".into()),
        };
        let strmap = |name: &str| -> Result<Vec<(String, String)>, String> {
            let mut v = vec![];
            if let Some(sec) = sections.get(name) {
                for (k, val) in sec {
                    match val {
                        Value::Str(s) => v.push((k.clone(), s.clone())),
                        _ => return Err(format!("[{name}] \"{k}\" must be a string")),
                    }
                }
            }
            Ok(v)
        };
        let text = strmap("text")?.into_iter().collect();
        let mut words = strmap("words")?;
        words.retain(|(k, _)| !k.is_empty());
        words.sort_by(|a, b| b.0.len().cmp(&a.0.len()).then(a.0.cmp(&b.0)));
        let name = if s("name").is_empty() {
            id.clone()
        } else {
            s("name")
        };
        Ok(Pack {
            id,
            name,
            subtitle: s("subtitle"),
            seed,
            sections,
            text,
            words,
        })
    }

    pub fn get(&self, section: &str, key: &str) -> Option<&Value> {
        self.sections.get(section)?.get(key)
    }
    pub fn str(&self, section: &str, key: &str) -> Option<&str> {
        match self.get(section, key)? {
            Value::Str(s) => Some(s),
            _ => None,
        }
    }
    pub fn int(&self, section: &str, key: &str) -> Option<i64> {
        match self.get(section, key)? {
            Value::Int(n) => Some(*n),
            _ => None,
        }
    }
    pub fn strs(&self, section: &str, key: &str) -> Option<Vec<String>> {
        match self.get(section, key)? {
            Value::List(v) => {
                let out: Vec<String> = v
                    .iter()
                    .filter_map(|x| match x {
                        Value::Str(s) => Some(s.clone()),
                        _ => None,
                    })
                    .collect();
                (!out.is_empty()).then_some(out)
            }
            _ => None,
        }
    }
    /// `"#rrggbb"` or `"#rrggbbaa"` as 0xRRGGBBAA
    pub fn color(&self, section: &str, key: &str) -> Option<u32> {
        parse_color(self.str(section, key)?)
    }
    /// ids under a prefix, e.g. every `[mon.*]`
    pub fn ids(&self, prefix: &str) -> Vec<&str> {
        let p = format!("{prefix}.");
        let mut v: Vec<&str> = self
            .sections
            .keys()
            .filter_map(|k| k.strip_prefix(p.as_str()))
            .collect();
        v.sort();
        v
    }

    /// Shown text as this pack words it: an exact replacement, then the
    /// in-text replacements.
    pub fn tr(&self, s: &str) -> String {
        let mut out = match self.text.get(s) {
            Some(r) => r.clone(),
            None => s.to_string(),
        };
        if self.words.is_empty() {
            return out;
        }
        // one left-to-right pass, so a replacement is never replaced again
        let mut res = String::with_capacity(out.len());
        let mut i = 0;
        'outer: while i < out.len() {
            for (from, to) in &self.words {
                if out[i..].starts_with(from.as_str()) {
                    res.push_str(to);
                    i += from.len();
                    continue 'outer;
                }
            }
            let c = out[i..].chars().next().unwrap();
            res.push(c);
            i += c.len_utf8();
        }
        out = res;
        out
    }

    /// The pack's name and subtitle for the host, as JSON.
    pub fn info_json(&self) -> String {
        format!(
            "{{\"id\":\"{}\",\"name\":\"{}\",\"subtitle\":\"{}\",\"seed\":{}}}",
            json_esc(&self.id),
            json_esc(&self.name),
            json_esc(&self.subtitle),
            self.seed.map(|s| s.to_string()).unwrap_or("null".into())
        )
    }
}

pub fn parse_color(s: &str) -> Option<u32> {
    let h = s.strip_prefix('#')?;
    let v = u32::from_str_radix(h, 16).ok()?;
    match h.len() {
        6 => Some(v << 8 | 0xff),
        8 => Some(v),
        _ => None,
    }
}

fn json_esc(s: &str) -> String {
    let mut o = String::new();
    for c in s.chars() {
        match c {
            '"' => o.push_str("\\\""),
            '\\' => o.push_str("\\\\"),
            '\n' => o.push_str("\\n"),
            c if (c as u32) < 0x20 => o.push_str(&format!("\\u{:04x}", c as u32)),
            c => o.push(c),
        }
    }
    o
}

// ---------------------------------------------------------------- the active pack
static ACTIVE: OnceLock<Pack> = OnceLock::new();

/// The pack in play; the engine's own world unless the host set another.
pub fn get() -> &'static Pack {
    ACTIVE.get_or_init(builtin)
}

/// Put a pack in play. Must come before the world is first built: the game's
/// tables are made from the pack once and kept.
pub fn set(src: &str) -> Result<(), String> {
    let p = Pack::parse(src)?;
    ACTIVE
        .set(p)
        .map_err(|_| "a world pack is already in play; reload to change it".to_string())
}

/// Shown text as the active pack words it.
pub fn t(s: &str) -> String {
    get().tr(s)
}

/// As [`t`], for labels that live for the whole game (place names, the
/// periods of the day): each distinct label is translated once and kept.
pub fn ts(s: &'static str) -> &'static str {
    let p = get();
    if p.text.is_empty() && p.words.is_empty() {
        return s;
    }
    static CACHE: OnceLock<Mutex<HashMap<&'static str, &'static str>>> = OnceLock::new();
    let mut c = CACHE
        .get_or_init(|| Mutex::new(HashMap::new()))
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    if let Some(r) = c.get(s) {
        return r;
    }
    let r = p.tr(s);
    let r: &'static str = if r == s { s } else { leak(r) };
    c.insert(s, r);
    r
}

pub fn leak(s: String) -> &'static str {
    Box::leak(s.into_boxed_str())
}
pub fn leak_list(v: Vec<String>) -> &'static [&'static str] {
    Box::leak(
        v.into_iter()
            .map(leak)
            .collect::<Vec<_>>()
            .into_boxed_slice(),
    )
}

/// The world's pixel font has only printable ASCII: fold the accented letters
/// of a pack's names (Annúminas, Dúnedain…) to plain ones for overhead speech.
pub fn ascii_fold(s: &str) -> std::borrow::Cow<'_, str> {
    if s.is_ascii() {
        return std::borrow::Cow::Borrowed(s);
    }
    let mut o = String::with_capacity(s.len());
    for c in s.chars() {
        if c.is_ascii() {
            o.push(c);
            continue;
        }
        let r = match c {
            'á' | 'à' | 'â' | 'ä' | 'ã' | 'å' | 'ā' => "a",
            'Á' | 'À' | 'Â' | 'Ä' | 'Ã' | 'Å' | 'Ā' => "A",
            'é' | 'è' | 'ê' | 'ë' | 'ē' => "e",
            'É' | 'È' | 'Ê' | 'Ë' | 'Ē' => "E",
            'í' | 'ì' | 'î' | 'ï' | 'ī' => "i",
            'Í' | 'Ì' | 'Î' | 'Ï' | 'Ī' => "I",
            'ó' | 'ò' | 'ô' | 'ö' | 'õ' | 'ō' | 'ø' => "o",
            'Ó' | 'Ò' | 'Ô' | 'Ö' | 'Õ' | 'Ō' | 'Ø' => "O",
            'ú' | 'ù' | 'û' | 'ü' | 'ū' => "u",
            'Ú' | 'Ù' | 'Û' | 'Ü' | 'Ū' => "U",
            'ý' | 'ÿ' => "y",
            'ñ' => "n",
            'Ñ' => "N",
            'ç' => "c",
            'Ç' => "C",
            'þ' => "th",
            'Þ' => "Th",
            'ð' => "dh",
            'Ð' => "Dh",
            'æ' => "ae",
            'Æ' => "Ae",
            'œ' => "oe",
            'ß' => "ss",
            '‘' | '’' => "'",
            '“' | '”' => "\"",
            '–' | '—' => "-",
            '…' => "...",
            _ => "?",
        };
        o.push_str(r);
    }
    std::borrow::Cow::Owned(o)
}

// ---------------------------------------------------------------- TOML subset
struct P<'a> {
    s: &'a [u8],
    i: usize,
    line: usize,
}

impl<'a> P<'a> {
    fn err(&self, m: &str) -> String {
        format!("line {}: {m}", self.line)
    }
    fn peek(&self) -> Option<u8> {
        self.s.get(self.i).copied()
    }
    fn bump(&mut self) -> Option<u8> {
        let c = self.peek()?;
        self.i += 1;
        if c == b'\n' {
            self.line += 1;
        }
        Some(c)
    }
    /// spaces and tabs (and newlines and comments when `nl`)
    fn ws(&mut self, nl: bool) {
        while let Some(c) = self.peek() {
            match c {
                b' ' | b'\t' | b'\r' => {
                    self.bump();
                }
                b'\n' if nl => {
                    self.bump();
                }
                b'#' => {
                    while let Some(c) = self.peek() {
                        if c == b'\n' {
                            break;
                        }
                        self.bump();
                    }
                }
                _ => break,
            }
        }
    }
    fn end_of_line(&mut self) -> Result<(), String> {
        self.ws(false);
        match self.peek() {
            None => Ok(()),
            Some(b'\n') => {
                self.bump();
                Ok(())
            }
            Some(_) => Err(self.err("unexpected text after the value")),
        }
    }
    fn bare(&mut self) -> String {
        let st = self.i;
        while let Some(c) = self.peek() {
            if c.is_ascii_alphanumeric() || c == b'_' || c == b'-' {
                self.bump();
            } else {
                break;
            }
        }
        String::from_utf8_lossy(&self.s[st..self.i]).into_owned()
    }
    fn string(&mut self) -> Result<String, String> {
        self.bump(); // opening quote
        let mut out: Vec<u8> = vec![];
        loop {
            if matches!(self.peek(), None | Some(b'\n')) {
                return Err(self.err("unterminated string"));
            }
            match self.bump() {
                None | Some(b'\n') => unreachable!(),
                Some(b'"') => break,
                Some(b'\\') => match self.bump() {
                    Some(b'"') => out.push(b'"'),
                    Some(b'\\') => out.push(b'\\'),
                    Some(b'n') => out.push(b'\n'),
                    Some(b't') => out.push(b'\t'),
                    Some(b'u') => {
                        let braced = self.peek() == Some(b'{');
                        if braced {
                            self.bump();
                        }
                        let st = self.i;
                        while self.peek().is_some_and(|c| c.is_ascii_hexdigit()) {
                            self.bump();
                        }
                        let hex = String::from_utf8_lossy(&self.s[st..self.i]).into_owned();
                        if braced && self.bump() != Some(b'}') {
                            return Err(self.err("bad \\u{…} escape"));
                        }
                        let ch = u32::from_str_radix(&hex, 16)
                            .ok()
                            .and_then(char::from_u32)
                            .ok_or_else(|| self.err("bad \\u escape"))?;
                        let mut b = [0u8; 4];
                        out.extend_from_slice(ch.encode_utf8(&mut b).as_bytes());
                    }
                    _ => return Err(self.err("unknown escape in string")),
                },
                Some(c) => out.push(c),
            }
        }
        String::from_utf8(out).map_err(|_| self.err("string is not valid UTF-8"))
    }
    fn key(&mut self) -> Result<String, String> {
        match self.peek() {
            Some(b'"') => self.string(),
            _ => {
                let k = self.bare();
                if k.is_empty() {
                    Err(self.err("expected a key"))
                } else {
                    Ok(k)
                }
            }
        }
    }
    fn value(&mut self) -> Result<Value, String> {
        match self.peek() {
            Some(b'"') => Ok(Value::Str(self.string()?)),
            Some(b'[') => {
                self.bump();
                let mut v = vec![];
                loop {
                    self.ws(true);
                    if self.peek() == Some(b']') {
                        self.bump();
                        break;
                    }
                    v.push(self.value()?);
                    self.ws(true);
                    match self.bump() {
                        Some(b',') => {}
                        Some(b']') => break,
                        _ => return Err(self.err("expected ',' or ']' in a list")),
                    }
                }
                Ok(Value::List(v))
            }
            Some(c) if c == b'-' || c == b'+' || c.is_ascii_digit() => {
                let st = self.i;
                self.bump();
                while self.peek().is_some_and(|c| c.is_ascii_digit() || c == b'_') {
                    self.bump();
                }
                let t: String = String::from_utf8_lossy(&self.s[st..self.i]).replace('_', "");
                t.parse()
                    .map(Value::Int)
                    .map_err(|_| self.err("bad number"))
            }
            _ => match self.bare().as_str() {
                "true" => Ok(Value::Bool(true)),
                "false" => Ok(Value::Bool(false)),
                _ => Err(self.err("expected a string, number, true/false or [list]")),
            },
        }
    }
}

pub fn parse_toml(src: &str) -> Result<HashMap<String, HashMap<String, Value>>, String> {
    let mut p = P {
        s: src.as_bytes(),
        i: 0,
        line: 1,
    };
    let mut out: HashMap<String, HashMap<String, Value>> = HashMap::new();
    let mut sec = String::new();
    loop {
        p.ws(true);
        let Some(c) = p.peek() else { break };
        if c == b'[' {
            p.bump();
            let mut parts = vec![];
            loop {
                p.ws(false);
                parts.push(p.key()?);
                p.ws(false);
                match p.bump() {
                    Some(b'.') => {}
                    Some(b']') => break,
                    _ => return Err(p.err("expected '.' or ']' in a section name")),
                }
            }
            sec = parts.join(".");
            out.entry(sec.clone()).or_default();
            p.end_of_line()?;
            continue;
        }
        let k = p.key()?;
        p.ws(false);
        if p.bump() != Some(b'=') {
            return Err(p.err(&format!("expected '=' after \"{k}\"")));
        }
        p.ws(false);
        let v = p.value()?;
        let tbl = out.entry(sec.clone()).or_default();
        if tbl.insert(k.clone(), v).is_some() {
            return Err(p.err(&format!("\"{k}\" is given twice")));
        }
        p.end_of_line()?;
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r##"
# a test world
[pack]
id = "test-world"
name = "Test Wórld"
seed = 1_409

[mon.wolf]
name = "a warg"   # comment after a value
color = "#4a4038"

[vendor.guard]
names = [
  "Ned",
  "Tom", # trailing comma next
]

[text]
"Graveyard" = "Barrow-downs"
"Say \"hi\"" = "Say é"

[words]
"ankh" = "stone"
"ankh shrine" = "waystone"
"##;

    #[test]
    fn parses_the_subset() {
        let p = Pack::parse(SAMPLE).unwrap();
        assert_eq!(p.id, "test-world");
        assert_eq!(p.name, "Test Wórld");
        assert_eq!(p.seed, Some(1409));
        assert_eq!(p.str("mon.wolf", "name"), Some("a warg"));
        assert_eq!(p.color("mon.wolf", "color"), Some(0x4a4038ff));
        assert_eq!(
            p.strs("vendor.guard", "names"),
            Some(vec!["Ned".into(), "Tom".into()])
        );
        assert_eq!(p.ids("mon"), vec!["wolf"]);
        assert_eq!(p.tr("Say \"hi\""), "Say é");
    }

    #[test]
    fn replaces_text_then_words_longest_first() {
        let p = Pack::parse(SAMPLE).unwrap();
        assert_eq!(p.tr("Graveyard"), "Barrow-downs");
        assert_eq!(p.tr("Find an ankh shrine."), "Find an waystone.");
        assert_eq!(p.tr("the ankh"), "the stone");
        // a replacement is not replaced again
        let q = Pack::parse("[pack]\nid=\"q\"\n[words]\n\"a\"=\"aa\"\n").unwrap();
        assert_eq!(q.tr("banana"), "baanaanaa");
    }

    #[test]
    fn reports_errors_with_lines() {
        assert!(Pack::parse("[text]\n\"a\" = \"b\"\n")
            .unwrap_err()
            .contains("[pack]"));
        let e = Pack::parse("[pack]\nid = \"x\"\nname = \"oops\n").unwrap_err();
        assert!(e.starts_with("line 3"), "{e}");
        let e = Pack::parse("[pack]\nid = \"Bad Id\"\n").unwrap_err();
        assert!(e.contains("lowercase"), "{e}");
        let e = Pack::parse("[pack]\nid = \"x\"\nid = \"y\"\n").unwrap_err();
        assert!(e.contains("twice"), "{e}");
    }

    #[test]
    fn folds_accents_for_the_pixel_font() {
        assert_eq!(
            ascii_fold("Annúminas – Dúnedain…"),
            "Annuminas - Dunedain..."
        );
    }
}
