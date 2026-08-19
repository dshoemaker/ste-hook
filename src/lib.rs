pub mod blocks;
pub mod rules;

use serde::Serialize;

#[derive(Serialize)]
pub struct Diagnostic {
    pub path: String,
    pub line: usize,
    pub column: usize,
    pub severity: &'static str,
    pub code: &'static str,
    pub message: String,
    pub help: &'static str,
}

pub fn severity_of(code: &str) -> &'static str {
    match code {
        "RED001" | "RED002" | "STE001" | "STE002" | "STE006" | "STE007" => "error",
        _ => "warning",
    }
}

pub fn lint_source(
    path: &str,
    source: &str,
    enabled: &[String],
    limits: &rules::Limits,
) -> Vec<Diagnostic> {
    let language = match blocks::language_for(path) {
        Some(l) => l,
        None => return Vec::new(),
    };
    let mut out = Vec::new();
    for block in blocks::blocks(source, language) {
        if block.ignore_all {
            continue;
        }
        let ctx = rules::Context {
            is_doc: block.is_doc,
            attached_code: block.attached_code.as_deref(),
        };
        for f in rules::check(&block.text, limits, enabled, &ctx) {
            if block.ignore.iter().any(|c| c == f.code) {
                continue;
            }
            // Map the offset in the joined paragraph back to the real
            // line and column it came from.
            let (row, col) = block
                .map
                .get(f.start)
                .copied()
                .unwrap_or_else(|| block.map.first().copied().unwrap_or((0, 0)));
            out.push(Diagnostic {
                path: path.to_string(),
                line: row + 1,
                column: col + 1,
                severity: severity_of(f.code),
                code: f.code,
                message: f.message,
                help: f.help,
            });
        }
    }
    out
}

pub fn lint_file(path: &str, enabled: &[String], limits: &rules::Limits) -> Vec<Diagnostic> {
    let source = match std::fs::read_to_string(path) {
        Ok(s) => s,
        Err(_) => return Vec::new(),
    };
    lint_source(path, &source, enabled, limits)
}
