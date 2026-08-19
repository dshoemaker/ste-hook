mod blocks;
mod rules;

use serde::Serialize;
use std::io::{self, BufRead, Read, Write};
use std::process::ExitCode;

#[derive(Serialize)]
struct Diagnostic {
    path: String,
    line: usize,
    column: usize,
    severity: &'static str,
    code: &'static str,
    message: String,
    help: &'static str,
}

fn severity_of(code: &str) -> &'static str {
    match code {
        "STE001" | "STE002" | "STE007" => "error",
        _ => "warning",
    }
}

fn lint_file(path: &str, enabled: &[String], limits: &rules::Limits) -> Vec<Diagnostic> {
    let language = match blocks::language_for(path) {
        Some(l) => l,
        None => return Vec::new(),
    };
    let source = match std::fs::read_to_string(path) {
        Ok(s) => s,
        Err(_) => return Vec::new(),
    };
    let mut out = Vec::new();
    for block in blocks::blocks(&source, language) {
        if block.ignore_all {
            continue;
        }
        for f in rules::check(&block.text, limits, enabled) {
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

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut paths = Vec::new();
    let mut jsonl = false;
    let mut agent = false;
    let mut enabled: Vec<String> = Vec::new();
    let mut limits = rules::Limits::default();
    let mut from_stdin = false;

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--format" if i + 1 < args.len() => {
                jsonl = args[i + 1] == "jsonl" || args[i + 1] == "json";
                agent = args[i + 1] == "agent";
                i += 1;
            }
            "--files0-from" if i + 1 < args.len() => {
                from_stdin = args[i + 1] == "-";
                i += 1;
            }
            "--rules" if i + 1 < args.len() => {
                enabled = args[i + 1].split(',').map(|s| s.trim().to_string()).collect();
                i += 1;
            }
            "--mode" if i + 1 < args.len() => {
                if args[i + 1] == "procedure" {
                    limits.descriptive_words = limits.procedural_words;
                }
                i += 1;
            }
            "--help" | "-h" => {
                eprintln!("comment-lint [--rules STE001,STE006] [--format text|jsonl|agent] [--files0-from -] <files...>");
                return ExitCode::SUCCESS;
            }
            other => paths.push(other.to_string()),
        }
        i += 1;
    }

    if from_stdin {
        let mut buf = Vec::new();
        if io::stdin().lock().read_to_end(&mut buf).is_ok() {
            for chunk in buf.split(|b| *b == 0) {
                if chunk.is_empty() {
                    continue;
                }
                paths.push(String::from_utf8_lossy(chunk).into_owned());
            }
        }
    } else if paths.is_empty() {
        for line in io::stdin().lock().lines().flatten() {
            if !line.trim().is_empty() {
                paths.push(line);
            }
        }
    }

    let mut all = Vec::new();
    for path in &paths {
        all.extend(lint_file(path, &enabled, &limits));
    }

    let stdout = io::stdout();
    let mut w = stdout.lock();
    if agent {
        // Deliberately reports the rule and the offending span only.
        // It never emits replacement prose: the point is that the author
        // rewrites the comment, not that a tool rewrites it for them.
        let _ = writeln!(
            w,
            "{} comment issue(s) found. Rewrite the comments below to satisfy \
             ASD-STE100. Do not delete the comments and do not weaken them to \
             pass the check.\n",
            all.len()
        );
        let mut last = String::new();
        for d in &all {
            if d.path != last {
                let _ = writeln!(w, "{}", d.path);
                last = d.path.clone();
            }
            let _ = writeln!(w, "  {}:{} [{}] {}", d.line, d.column, d.code, d.message);
            let _ = writeln!(w, "      -> {}", d.help);
        }
        let _ = writeln!(
            w,
            "\nIf a finding genuinely does not apply, put a directive comment on \
             the line above the block:\n  # ste:ignore STE001    (or bare `ste:ignore` \
             for the whole block)\nUse this sparingly and only with a reason."
        );
    } else if jsonl {
        for d in &all {
            let _ = writeln!(w, "{}", serde_json::to_string(d).unwrap_or_default());
        }
    } else {
        for d in &all {
            let _ = writeln!(w, "{}:{}:{}: {} [{}] {}", d.path, d.line, d.column, d.severity, d.code, d.message);
            let _ = writeln!(w, "    help: {}", d.help);
        }
    }

    if all.is_empty() { ExitCode::SUCCESS } else { ExitCode::from(1) }
}
