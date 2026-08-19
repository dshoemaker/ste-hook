use comment_lint::{lint_file, rules};
use std::io::{self, BufRead, Read, Write};
use std::process::ExitCode;

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
                eprintln!("comment-lint [--rules RED001,RED002,STE001,STE006] [--format text|jsonl|agent] [--files0-from -] <files...>");
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
            "{} comment issue(s) found.\n\
             For RED001/RED002 (redundant or narration comments): delete the \
             comment, or replace it with rationale the code cannot express.\n\
             For STE rules: rewrite the comment. Do not delete it and do not \
             strip it down to something uninformative to pass the check.\n",
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
             the line above the block:\n  # comment-lint:ignore STE001    (or bare \
             `comment-lint:ignore` for the whole block)\nUse this sparingly and only \
             with a reason."
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
