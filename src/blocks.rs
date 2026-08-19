use tree_sitter::{Language, Node, Parser};

/// A run of consecutive line comments, joined into one logical paragraph.
pub struct Block {
    pub text: String,
    /// One entry per byte of `text`: (line, column) in the original file.
    pub map: Vec<(usize, usize)>,
    /// Codes suppressed by a preceding `ste:ignore` directive. Empty vec with
    /// `ignore_all` set means every code is suppressed.
    pub ignore: Vec<String>,
    pub ignore_all: bool,
}

/// `# ste:ignore` suppresses everything in the next block.
/// `# ste:ignore STE001,STE006` suppresses only those codes.
/// This is the escape hatch that keeps a blocking hook from looping forever
/// on a comment that genuinely cannot comply.
fn parse_ignore(body: &str) -> Option<(Vec<String>, bool)> {
    let t = body.trim();
    let rest = t
        .strip_prefix("ste:ignore")
        .or_else(|| t.strip_prefix("ste-ignore"))
        .or_else(|| t.strip_prefix("ste:disable"))?;
    let rest = rest.trim_start_matches(':').trim();
    if rest.is_empty() {
        return Some((Vec::new(), true));
    }
    let codes: Vec<String> = rest
        .split(&[',', ' '][..])
        .map(|c| c.trim().to_ascii_uppercase())
        .filter(|c| !c.is_empty())
        .collect();
    Some((codes, false))
}

pub fn language_for(path: &str) -> Option<Language> {
    let ext = path.rsplit('.').next()?;
    match ext {
        "rb" | "rake" | "gemspec" => Some(tree_sitter_ruby::language()),
        "js" | "jsx" | "mjs" | "cjs" => Some(tree_sitter_javascript::language()),
        "ts" => Some(tree_sitter_typescript::language_typescript()),
        "tsx" => Some(tree_sitter_typescript::language_tsx()),
        _ => None,
    }
}

fn collect<'a>(node: Node<'a>, out: &mut Vec<Node<'a>>) {
    if node.kind() == "comment" {
        out.push(node);
        return;
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect(child, out);
    }
}

/// Comment lines that are directives, borders, or commented-out code are not
/// prose. Skipping them also breaks the surrounding block, which is correct:
/// a `# rubocop:disable` between two sentences is a real boundary.
fn is_noise(body: &str) -> bool {
    let t = body.trim();
    if t.is_empty() {
        return true;
    }
    let lowered = t.to_ascii_lowercase();
    const DIRECTIVES: [&str; 12] = [
        "rubocop:", "eslint-", "@ts-", "prettier-", "biome-ignore", "frozen_string_literal:",
        "encoding:", "istanbul ", "c8 ", "v8 ", ":nodoc:", "type:",
    ];
    if DIRECTIVES.iter().any(|d| lowered.starts_with(d)) || t.starts_with('@') {
        return true;
    }
    // Border comments: ####, ----, ====
    if t.chars().all(|c| "#-=*_/".contains(c)) {
        return true;
    }
    // Commented-out code.
    const CODEISH: [&str; 12] = [
        "def ", "class ", "module ", "end", "function ", "const ", "let ", "var ",
        "return ", "import ", "export ", "}",
    ];
    if CODEISH.iter().any(|c| t.starts_with(c)) {
        return true;
    }
    t.ends_with(';') || t.ends_with('{')
}

fn strip_marker(raw: &str) -> (usize, &str) {
    let after = raw
        .strip_prefix("///")
        .or_else(|| raw.strip_prefix("//"))
        .or_else(|| raw.strip_prefix("#!"))
        .or_else(|| raw.strip_prefix('#'))
        .unwrap_or(raw);
    let trimmed = after.trim_start();
    (raw.len() - trimmed.len(), trimmed)
}

pub fn blocks(source: &str, language: Language) -> Vec<Block> {
    let mut parser = Parser::new();
    if parser.set_language(language).is_err() {
        return Vec::new();
    }
    let Some(tree) = parser.parse(source, None) else {
        return Vec::new();
    };

    let mut nodes = Vec::new();
    collect(tree.root_node(), &mut nodes);
    nodes.sort_by_key(|n| (n.start_position().row, n.start_position().column));

    let lines: Vec<&str> = source.lines().collect();
    let mut out = Vec::new();
    let mut run: Vec<(usize, usize, String)> = Vec::new();
    let mut pending: Option<(Vec<String>, bool)> = None;

    let flush = |run: &mut Vec<(usize, usize, String)>,
                 out: &mut Vec<Block>,
                 pending: &mut Option<(Vec<String>, bool)>| {
        if run.is_empty() {
            return;
        }
        let mut text = String::new();
        let mut map: Vec<(usize, usize)> = Vec::new();
        for (row, col, body) in run.iter() {
            if !text.is_empty() {
                text.push(' ');
                map.push(*map.last().unwrap());
            }
            for (i, ch) in body.char_indices() {
                let mut buf = [0u8; 4];
                for _ in ch.encode_utf8(&mut buf).bytes() {
                    map.push((*row, col + i));
                }
                text.push(ch);
            }
        }
        run.clear();
        if !text.trim().is_empty() {
            let (ignore, ignore_all) = pending.take().unwrap_or((Vec::new(), false));
            out.push(Block { text, map, ignore, ignore_all });
        }
    };

    for node in nodes {
        let start = node.start_position();
        let end = node.end_position();
        let raw = node.utf8_text(source.as_bytes()).unwrap_or("");

        // A comment with code before it on the same line is a trailing note,
        // not part of a block.
        let prefix = lines
            .get(start.row)
            .map(|l| &l[..start.column.min(l.len())])
            .unwrap_or("");
        let standalone = prefix.trim().is_empty();

        // Multi-line comments (/* */, =begin) are their own block.
        if end.row != start.row {
            flush(&mut run, &mut out, &mut pending);
            let body: String = raw
                .lines()
                .map(|l| {
                    l.trim()
                        .trim_start_matches("/*")
                        .trim_start_matches("*/")
                        .trim_start_matches('*')
                        .trim_end_matches("*/")
                        .trim()
                })
                .filter(|l| !l.is_empty() && *l != "=begin" && *l != "=end")
                .collect::<Vec<_>>()
                .join(" ");
            if !body.trim().is_empty() && standalone {
                let map = vec![(start.row, start.column); body.len()];
                let (ignore, ignore_all) = pending.take().unwrap_or((Vec::new(), false));
                out.push(Block { text: body, map, ignore, ignore_all });
            }
            continue;
        }

        let (lead, body) = strip_marker(raw);
        if let Some(directive) = parse_ignore(body) {
            flush(&mut run, &mut out, &mut pending);
            pending = Some(directive);
            continue;
        }
        if !standalone || is_noise(body) {
            flush(&mut run, &mut out, &mut pending);
            continue;
        }

        // The joining predicate: adjacent line, identical column.
        if let Some((prow, pcol, _)) = run.last() {
            if *prow + 1 != start.row || *pcol != start.column {
                flush(&mut run, &mut out, &mut pending);
            }
        }
        run.push((start.row, start.column + lead, body.to_string()));
    }
    flush(&mut run, &mut out, &mut pending);
    out
}
