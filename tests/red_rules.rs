//! RED rule family (narration detection) and position-aware STE001.
//! Written before the implementation (TDD).

use comment_lint::rules::Limits;
use comment_lint::{lint_source, Diagnostic};

fn lint(path: &str, source: &str, rules: &[&str]) -> Vec<Diagnostic> {
    let enabled: Vec<String> = rules.iter().map(|s| s.to_string()).collect();
    lint_source(path, source, &enabled, &Limits::default())
}

fn codes(d: &[Diagnostic]) -> Vec<&'static str> {
    d.iter().map(|f| f.code).collect()
}

// --- RED002: narration / change-log phrasing -------------------------------

#[test]
fn red002_flags_now_uses() {
    let d = lint("a.rb", "# Now uses the connection pool.\nx = 1\n", &["RED002"]);
    assert_eq!(codes(&d), vec!["RED002"]);
}

#[test]
fn red002_flags_previously() {
    let d = lint("a.rb", "# Previously this returned nil.\nx = 1\n", &["RED002"]);
    assert_eq!(codes(&d), vec!["RED002"]);
}

#[test]
fn red002_flags_updated_to_use() {
    let d = lint("a.rb", "# Updated to use the new API surface.\nx = 1\n", &["RED002"]);
    assert_eq!(codes(&d), vec!["RED002"]);
}

#[test]
fn red002_flags_as_requested() {
    let d = lint("a.rb", "# As requested, retry twice on timeout.\nx = 1\n", &["RED002"]);
    assert_eq!(codes(&d), vec!["RED002"]);
}

#[test]
fn red002_flags_this_change() {
    let d = lint("a.rb", "# This change makes the pool lazy.\nx = 1\n", &["RED002"]);
    assert_eq!(codes(&d), vec!["RED002"]);
}

#[test]
fn red002_flags_process_narration() {
    let d = lint("a.rb", "# First we warm the cache.\nx = 1\n", &["RED002"]);
    assert_eq!(codes(&d), vec!["RED002"]);
}

#[test]
fn red002_flags_commit_verb_sentence_start() {
    let d = lint("a.rb", "# Fixed a race in the warm path.\nx = 1\n", &["RED002"]);
    assert_eq!(codes(&d), vec!["RED002"]);
}

#[test]
fn red002_allows_removed_as_adjective() {
    // "Removed entries" describes state, not an edit.
    let d = lint("a.rb", "# Removed entries expire after an hour.\nx = 1\n", &["RED002"]);
    assert!(d.is_empty(), "{:?}", codes(&d));
}

#[test]
fn red002_allows_present_tense_description() {
    let d = lint("a.rb", "# The pool retries twice on timeout.\nx = 1\n", &["RED002"]);
    assert!(d.is_empty());
}

#[test]
fn red002_ignores_phrases_inside_code_spans() {
    let d = lint("a.rb", "# Set `mode: \"now uses\"` before boot.\nx = 1\n", &["RED002"]);
    assert!(d.is_empty());
}

// --- RED001: comment restates the adjacent code ----------------------------

#[test]
fn red001_flags_body_comment_restating_next_statement() {
    let src = "def run\n  # Find the widget by slug.\n  find_widget_by_slug(slug)\nend\n";
    let d = lint("a.rb", src, &["RED001"]);
    assert_eq!(codes(&d), vec!["RED001"]);
    assert_eq!(d[0].line, 2);
}

#[test]
fn red001_flags_doc_comment_restating_definition() {
    let src = "# Find widget by slug.\ndef find_widget_by_slug(slug)\n  db[slug]\nend\n";
    let d = lint("a.rb", src, &["RED001"]);
    assert_eq!(codes(&d), vec!["RED001"]);
}

#[test]
fn red001_allows_surplus_information() {
    // Overlapping words plus real information must pass.
    let src = "def run\n  # Find the widget by slug, falling back to the numeric id.\n  find_widget_by_slug(slug)\nend\n";
    let d = lint("a.rb", src, &["RED001"]);
    assert!(d.is_empty(), "{:?}", codes(&d));
}

#[test]
fn red001_allows_rationale() {
    let src = "def run\n  # The cache misses on cold boot or the pool raises.\n  warm_cache\nend\n";
    let d = lint("a.rb", src, &["RED001"]);
    assert!(d.is_empty());
}

#[test]
fn red001_requires_adjacency() {
    // A blank line between comment and code detaches the block.
    let src = "def run\n  # Find the widget by slug.\n\n  find_widget_by_slug(slug)\nend\n";
    let d = lint("a.rb", src, &["RED001"]);
    assert!(d.is_empty());
}

#[test]
fn red001_works_in_javascript() {
    let src = "function run() {\n  // Parse the request body.\n  parseRequestBody(req)\n}\n";
    let d = lint("a.js", src, &["RED001"]);
    assert_eq!(codes(&d), vec!["RED001"]);
}

// --- STE001 position-awareness ----------------------------------------------

const LONG_26: &str = "The pool keeps ten warm connections around after boot because the very first busy request must not ever wait on a slow database handshake at all.";

#[test]
fn ste001_applies_to_doc_comments() {
    assert_eq!(LONG_26.split_whitespace().count(), 26);
    let src = format!("# {LONG_26}\ndef boot\n  x\nend\n");
    let d = lint("a.rb", &src, &["STE001"]);
    assert_eq!(codes(&d), vec!["STE001"]);
}

#[test]
fn ste001_exempts_body_comments() {
    let src = format!("def boot\n  # {LONG_26}\n  warm\nend\n");
    let d = lint("a.rb", &src, &["STE001"]);
    assert!(d.is_empty(), "body comments have no word limit: {:?}", codes(&d));
}

#[test]
fn ste001_exempts_file_level_comments() {
    let src = format!("# {LONG_26}\nx = 1\n");
    let d = lint("a.rb", &src, &["STE001"]);
    assert!(d.is_empty(), "file-level comments have no word limit");
}

// --- Directive rename --------------------------------------------------------

#[test]
fn comment_lint_ignore_scoped() {
    let src = "# comment-lint:ignore RED002\n# Now uses the pool.\nx = 1\n";
    let d = lint("a.rb", src, &["RED002"]);
    assert!(d.is_empty());
}

#[test]
fn comment_lint_ignore_blanket() {
    let src = "# comment-lint:ignore\n# Now uses the pool; previously it did not.\nx = 1\n";
    let d = lint("a.rb", src, &["RED002", "STE007"]);
    assert!(d.is_empty());
}

#[test]
fn old_ste_ignore_no_longer_suppresses() {
    let src = "# ste:ignore RED002\n# Now uses the pool.\nx = 1\n";
    let d = lint("a.rb", src, &["RED002"]);
    assert_eq!(codes(&d), vec!["RED002"], "legacy directive must not suppress");
}
