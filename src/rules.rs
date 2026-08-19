use regex::Regex;
use std::sync::OnceLock;

/// Passive-voice, complex-verb, -ing and imprecise-phrase heuristics are
/// ported from johnsaigle/ste-lint (MIT, (c) 2026 ste-lint contributors).
pub struct Finding {
    pub start: usize,
    pub len: usize,
    pub code: &'static str,
    pub message: String,
    pub help: &'static str,
}

fn re(cell: &'static OnceLock<Regex>, pattern: &str) -> &'static Regex {
    cell.get_or_init(|| Regex::new(pattern).expect("built-in pattern is valid"))
}

/// Blank out code so prose rules never see identifiers. Same length in, same
/// length out, so every offset stays valid.
pub fn mask(text: &str) -> String {
    static CELL: OnceLock<Regex> = OnceLock::new();
    let patterns = concat!(
        r"`[^`]*`",
        r"|https?://\S+",
        r"|\{[^}]*\}",
        r"|\b[A-Za-z_][A-Za-z0-9_]*(?:(?:::|\#|\.)[A-Za-z0-9_?!]+)+",
        r"|\b[a-z]+_[a-z0-9_]+\b",
        r"|\b[a-z]+[A-Z][A-Za-z0-9]*\b",
        r"|\b[A-Z]{2,}\b",
        r"|\b\d+(?:\.\d+)+\b",
        r"|\b[A-Za-z_][A-Za-z0-9_]*\([^)]*\)",
    );
    let rx = re(&CELL, patterns);
    let mut out = text.as_bytes().to_vec();
    for m in rx.find_iter(text) {
        for b in out[m.start()..m.end()].iter_mut() {
            *b = b'\0';
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

const ABBREV: [&str; 8] = ["e.g", "i.e", "vs", "etc", "cf", "Mr", "Dr", "al"];

/// Byte ranges of sentences. A period inside a masked span, or closing an
/// abbreviation, is not a boundary.
pub fn sentences(text: &str, masked: &str) -> Vec<(usize, usize)> {
    let bytes = text.as_bytes();
    let mask_bytes = masked.as_bytes();
    let mut out = Vec::new();
    let mut start = 0usize;
    for i in 0..bytes.len() {
        if !matches!(bytes[i], b'.' | b'!' | b'?') || mask_bytes[i] == 0 {
            continue;
        }
        let head = &text[start..=i];
        let abbrev = ABBREV.iter().any(|a| {
            head.trim_end_matches('.')
                .to_ascii_lowercase()
                .ends_with(&a.to_ascii_lowercase())
        });
        let next_ok = text[i + 1..]
            .chars()
            .find(|c| !c.is_whitespace())
            .map_or(true, |c| c.is_uppercase() || c.is_ascii_digit());
        if !abbrev && next_ok {
            out.push((start, i + 1));
            start = i + 1;
        }
    }
    if text[start..].trim() != "" {
        out.push((start, text.len()));
    }
    out
}

const IMPERATIVE: [&str; 30] = [
    "add", "call", "check", "clear", "close", "create", "delete", "disable", "do", "enable",
    "ensure", "find", "get", "handle", "install", "make", "move", "open", "put", "read", "remove",
    "reset", "return", "run", "set", "start", "stop", "update", "use", "write",
];

fn is_procedural(sentence: &str) -> bool {
    sentence
        .split_whitespace()
        .next()
        .map(|w| {
            let w: String = w
                .chars()
                .filter(|c| c.is_alphabetic())
                .collect::<String>()
                .to_ascii_lowercase();
            IMPERATIVE.contains(&w.as_str())
        })
        .unwrap_or(false)
}

const PHRASES: [(&str, &str); 10] = [
    (r"(?i)\bin order to\b", "to"),
    (r"(?i)\bdue to the fact that\b", "because"),
    (r"(?i)\ba number of\b", "several, or an exact number"),
    (r"(?i)\butili[sz]e(?:d|s|ing)?\b", "use"),
    (r"(?i)\bprior to\b", "before"),
    (r"(?i)\bsubsequent to\b", "after"),
    (r"(?i)\b(?:obviously|clearly|simply|basically)\b", "a specific explanation"),
    (r"(?i)\b(?:seamless|robust|crucial)\b", "a precise, measurable term"),
    (r"(?i)\b(?:delve|delves|delved|delving)\b", "examine"),
    (r"(?i)\b(?:moreover|additionally)\b", "a direct transition, or none"),
];

const ING_ALLOW: [&str; 16] = [
    "thing", "things", "string", "strings", "during", "nothing", "something", "anything",
    "everything", "warning", "setting", "settings", "mapping", "encoding", "logging", "meaning",
];

pub struct Limits {
    pub procedural_words: usize,
    pub descriptive_words: usize,
    pub max_sentences: usize,
}

impl Default for Limits {
    fn default() -> Self {
        Self { procedural_words: 20, descriptive_words: 25, max_sentences: 6 }
    }
}

/// Where the block sits in the tree. Word limits apply only in doc
/// position; the redundancy rule needs the adjacent code's header line.
pub struct Context<'a> {
    pub is_doc: bool,
    pub attached_code: Option<&'a str>,
}

/// Function words carrying no content; excluded before the overlap test.
const STOPWORDS: [&str; 40] = [
    "the", "a", "an", "to", "of", "in", "on", "for", "and", "or", "not", "by", "at", "it", "its",
    "this", "that", "these", "those", "with", "from", "as", "is", "are", "was", "were", "be",
    "been", "we", "you", "they", "if", "then", "else", "when", "into", "via", "per", "each", "all",
];

/// Light suffix stripping so "finds" matches "find" and "parsing" "parse".
fn stem(w: &str) -> &str {
    for (suffix, min_len) in [("ing", 6), ("ed", 5), ("es", 5), ("s", 4)] {
        if w.len() >= min_len && w.ends_with(suffix) {
            return &w[..w.len() - suffix.len()];
        }
    }
    w
}

/// snake_case and camelCase identifiers split into lowercase word parts.
fn split_ident(token: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut cur = String::new();
    for ch in token.chars() {
        if ch == '_' {
            if !cur.is_empty() {
                parts.push(cur.to_ascii_lowercase());
                cur = String::new();
            }
        } else if ch.is_uppercase() && cur.chars().last().is_some_and(|c| c.is_lowercase()) {
            parts.push(cur.to_ascii_lowercase());
            cur = ch.to_string();
        } else {
            cur.push(ch);
        }
    }
    if !cur.is_empty() {
        parts.push(cur.to_ascii_lowercase());
    }
    parts
}

/// Every word (and stem) present in the identifiers of a line of code.
fn identifier_words(code: &str) -> std::collections::HashSet<String> {
    static CELL: OnceLock<Regex> = OnceLock::new();
    let rx = re(&CELL, r"[A-Za-z][A-Za-z0-9_]*");
    let mut out = std::collections::HashSet::new();
    for m in rx.find_iter(code) {
        for part in split_ident(m.as_str()) {
            if part.len() >= 2 {
                out.insert(stem(&part).to_string());
                out.insert(part);
            }
        }
    }
    out
}

/// Change-log and process-narration phrasings. One finding per block is
/// enough: the fix is deleting or rewriting the whole comment.
const NARRATION: [&str; 9] = [
    r"(?i)\bnow\s+(?:uses|checks|returns|handles|calls|takes|supports|does|has|includes|works|also)\b",
    r"(?i)\bpreviously\b",
    r"(?i)\bno\s+longer\b",
    r"(?i)\bthis\s+(?:change|commit|pr|patch|update|refactor|edit)\b",
    r"(?i)\bas\s+requested\b",
    r"(?i)\bper\s+(?:the\s+)?(?:request|feedback|review|discussion)\b",
    r"(?i)\b(?:first|next|then|finally),?\s+we\b",
    r"(?i)\bupdated?\s+to\s+(?:use|support|handle|match)\b",
    r"(?i)\bchanged\s+(?:from|to)\b",
];

/// Commit-message verbs opening a sentence, followed by an object; guards
/// against adjectival reads like "Removed entries expire".
const COMMIT_VERB: &str = r"(?i)^\s*(?:added|removed|fixed|renamed|moved|changed|updated|refactored|improved)\s+(?:the|a|an|this|that|it|to|support|unused)\b";

pub fn check(text: &str, limits: &Limits, enabled: &[String], ctx: &Context) -> Vec<Finding> {
    let masked = mask(text);
    let sents = sentences(text, &masked);
    let mut out: Vec<Finding> = Vec::new();
    let on = |code: &str| enabled.is_empty() || enabled.iter().any(|e| e == code);

    // The redundancy test runs on the original text, not the masked one:
    // masking blanks exactly the identifiers the overlap needs.
    if on("RED001") {
        if let Some(code_line) = ctx.attached_code {
            let idents = identifier_words(code_line);
            static CELL: OnceLock<Regex> = OnceLock::new();
            let rx = re(&CELL, r"[A-Za-z]{2,}");
            let content: Vec<&str> = rx
                .find_iter(text)
                .map(|m| m.as_str())
                .filter(|w| !STOPWORDS.contains(&w.to_ascii_lowercase().as_str()))
                .collect();
            let all_restated = !content.is_empty()
                && !idents.is_empty()
                && content.iter().all(|w| {
                    let lw = w.to_ascii_lowercase();
                    idents.contains(&lw) || idents.contains(stem(&lw))
                });
            if all_restated {
                out.push(Finding {
                    start: 0,
                    len: text.len(),
                    code: "RED001",
                    message: "comment restates the adjacent code".into(),
                    help: "Delete it, or state what the code cannot say: why, constraints, units.",
                });
            }
        }
    }

    if on("RED002") {
        let mut hit: Option<(usize, usize)> = None;
        for pattern in NARRATION {
            let rx = Regex::new(pattern).expect("built-in pattern is valid");
            if let Some(m) = rx.find(&masked) {
                if hit.map_or(true, |(s, _)| m.start() < s) {
                    hit = Some((m.start(), m.len()));
                }
            }
        }
        if hit.is_none() {
            static CELL: OnceLock<Regex> = OnceLock::new();
            let rx = re(&CELL, COMMIT_VERB);
            for (s, e) in &sents {
                if let Some(m) = rx.find(&masked[*s..*e]) {
                    hit = Some((s + m.start(), m.len()));
                    break;
                }
            }
        }
        if let Some((start, len)) = hit {
            out.push(Finding {
                start,
                len,
                code: "RED002",
                message: "narration or change-log comment".into(),
                help: "Describe the code as it is, or delete the comment. Editing history belongs in the commit message.",
            });
        }
    }

    if on("STE001") && ctx.is_doc {
        for (s, e) in &sents {
            let body = &text[*s..*e];
            let n = body.split_whitespace().count();
            let max = if is_procedural(body) { limits.procedural_words } else { limits.descriptive_words };
            if n > max {
                out.push(Finding {
                    start: *s,
                    len: e - s,
                    code: "STE001",
                    message: format!("sentence has {n} words; the maximum is {max}"),
                    help: "Split the sentence into shorter statements.",
                });
            }
        }
    }

    if on("STE002") && sents.len() > limits.max_sentences {
        let (s, e) = sents[limits.max_sentences];
        out.push(Finding {
            start: s,
            len: e - s,
            code: "STE002",
            message: format!(
                "comment block has {} sentences; the maximum is {}",
                sents.len(),
                limits.max_sentences
            ),
            help: "Split the block so each paragraph has one topic.",
        });
    }

    // Scan the masked text so code never matches, but report against `text`.
    if on("STE003") {
        static CELL: OnceLock<Regex> = OnceLock::new();
        let rx = re(&CELL, r"(?i)\b(?:am|are|is|was|were|be|been|being)\s+(?:[a-z-]+\s+){0,2}[a-z-]+(?:ed|en)\b");
        for m in rx.find_iter(&masked) {
            out.push(Finding {
                start: m.start(),
                len: m.len(),
                code: "STE003",
                message: "possible passive voice".into(),
                help: "Name the actor and use active voice when the actor is known.",
            });
        }
    }

    if on("STE004") {
        static CELL: OnceLock<Regex> = OnceLock::new();
        let rx = re(&CELL, r"(?i)\b(?:(?:has|have|had)\s+(?:been\s+)?[a-z-]+(?:ed|en)|(?:will|would|could|should|may|might|must)\s+have\s+[a-z-]+(?:ed|en))\b");
        for m in rx.find_iter(&masked) {
            out.push(Finding {
                start: m.start(),
                len: m.len(),
                code: "STE004",
                message: "possible complex verb construction".into(),
                help: "Use a simple present, past, or future verb form.",
            });
        }
    }

    if on("STE005") {
        static CELL: OnceLock<Regex> = OnceLock::new();
        let rx = re(&CELL, r"(?i)\b[a-z][a-z-]{2,}ing\b");
        for m in rx.find_iter(&masked) {
            let word = &text[m.start()..m.end()];
            if ING_ALLOW.contains(&word.to_ascii_lowercase().as_str()) {
                continue;
            }
            let before = m.start().checked_sub(1).map(|i| masked.as_bytes()[i]);
            let after = masked.as_bytes().get(m.end()).copied();
            if before == Some(b'.') || after == Some(b'.') {
                continue;
            }
            out.push(Finding {
                start: m.start(),
                len: m.len(),
                code: "STE005",
                message: format!("review the -ing form {word:?}"),
                help: "Use an -ing form only as a technical noun.",
            });
        }
    }

    if on("STE006") {
        for (pattern, replacement) in PHRASES {
            let rx = Regex::new(pattern).expect("built-in phrase pattern is valid");
            for m in rx.find_iter(&masked) {
                out.push(Finding {
                    start: m.start(),
                    len: m.len(),
                    code: "STE006",
                    message: format!(
                        "imprecise phrase {:?}; try {replacement:?}",
                        &text[m.start()..m.end()]
                    ),
                    help: "Prefer a precise, concrete term.",
                });
            }
        }
    }

    if on("STE007") {
        for (i, b) in masked.bytes().enumerate() {
            if b == b';' {
                out.push(Finding {
                    start: i,
                    len: 1,
                    code: "STE007",
                    message: "semicolon joins instructions or topics".into(),
                    help: "Use separate sentences or a vertical list.",
                });
            }
        }
    }

    // "has been opened" matches both STE003 and STE004. Keep the more
    // specific complex-verb finding and drop the contained passive one.
    let complex: Vec<(usize, usize)> = out
        .iter()
        .filter(|f| f.code == "STE004")
        .map(|f| (f.start, f.start + f.len))
        .collect();
    out.retain(|f| {
        f.code != "STE003"
            || !complex.iter().any(|(s, e)| f.start >= *s && f.start + f.len <= *e)
    });

    out.sort_by_key(|f| f.start);
    out
}
