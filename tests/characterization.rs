//! Characterization tests: lock in the behaviour the handoff verified by
//! hand, before the tree-sitter migration or any rule changes.

use comment_lint::rules::{mask, Limits};
use comment_lint::{lint_source, Diagnostic};

fn lint_rb(source: &str, rules: &[&str]) -> Vec<Diagnostic> {
    let enabled: Vec<String> = rules.iter().map(|s| s.to_string()).collect();
    lint_source("test.rb", source, &enabled, &Limits::default())
}

fn lint_js(source: &str, rules: &[&str]) -> Vec<Diagnostic> {
    let enabled: Vec<String> = rules.iter().map(|s| s.to_string()).collect();
    lint_source("test.js", source, &enabled, &Limits::default())
}

/// Expected 1-based column of `needle` on 1-based `line` of `source`.
fn col_of(source: &str, line: usize, needle: &str) -> usize {
    source.lines().nth(line - 1).unwrap().find(needle).unwrap() + 1
}

#[test]
fn three_line_block_maps_finding_to_real_line_and_column() {
    let src = "x = 1\n\
               # The pool holds ten connections and it refills them at boot.\n\
               # The refill happens once.\n\
               # Use it in order to fetch rows.\n\
               y = 2\n";
    let d = lint_rb(src, &["STE006"]);
    assert_eq!(d.len(), 1, "one STE006 finding expected");
    assert_eq!(d[0].code, "STE006");
    assert_eq!(d[0].line, 4);
    assert_eq!(d[0].column, col_of(src, 4, "in order to"));
}

#[test]
fn indented_block_joins_and_maps_columns() {
    let src = "def a\n\
               \x20\x20# The helper runs twice per boot.\n\
               \x20\x20# It runs in order to warm the cache.\n\
               end\n";
    let d = lint_rb(src, &["STE006"]);
    assert_eq!(d.len(), 1);
    assert_eq!(d[0].line, 3);
    assert_eq!(d[0].column, col_of(src, 3, "in order to"));
}

#[test]
fn sentence_spanning_lines_is_one_sentence() {
    // Six short sentences split across lines must count as six, not more.
    let src = "# One a. Two b. Three\n\
               # c. Four d. Five e. Six\n\
               # f.\n\
               x = 1\n";
    let d = lint_rb(src, &["STE002"]);
    assert!(d.is_empty(), "six sentences across lines must pass: {:?}", msgs(&d));
}

#[test]
fn trailing_comment_never_joins_and_is_not_linted() {
    let src = "x = 1 # in order to test the trailer\n\
               # In order to check the standalone comment.\n";
    let d = lint_rb(src, &["STE006"]);
    assert_eq!(d.len(), 1, "only the standalone comment is linted");
    assert_eq!(d[0].line, 2);
}

#[test]
fn directive_line_breaks_a_block() {
    // Joined this would be seven sentences (STE002); the rubocop directive
    // splits it into four and three.
    let src = "# One a. Two b. Three c. Four d.\n\
               # rubocop:disable Foo/Bar\n\
               # Five e. Six f. Seven g.\n\
               x = 1\n";
    let d = lint_rb(src, &["STE002"]);
    assert!(d.is_empty(), "directive must break the block: {:?}", msgs(&d));
}

#[test]
fn code_heavy_comment_produces_zero_findings() {
    let src = "# Call `Widget#find_by_slug`. The method uses ActiveRecord::Base.connection.\nx = 1\n";
    let d = lint_rb(src, &[]);
    assert!(d.is_empty(), "masked spans must not trigger rules: {:?}", msgs(&d));
}

#[test]
fn semicolon_inside_masked_span_is_not_flagged() {
    let src = "# Call `foo(); bar()` at boot.\nx = 1\n";
    let d = lint_rb(src, &["STE007"]);
    assert!(d.is_empty());
}

#[test]
fn semicolon_in_prose_is_flagged() {
    let src = "# The pool boots; it refills.\nx = 1\n";
    let d = lint_rb(src, &["STE007"]);
    assert_eq!(d.len(), 1);
    assert_eq!(d[0].code, "STE007");
    assert_eq!(d[0].column, col_of(src, 1, ";"));
}

#[test]
fn scoped_ignore_suppresses_only_listed_codes() {
    let src = "# comment-lint:ignore STE007\n\
               # The pool boots; we go in order to warm it.\n\
               x = 1\n";
    let d = lint_rb(src, &["STE006", "STE007"]);
    assert_eq!(d.len(), 1, "STE007 suppressed, STE006 kept: {:?}", msgs(&d));
    assert_eq!(d[0].code, "STE006");
}

#[test]
fn blanket_ignore_suppresses_everything_for_next_block_only() {
    let src = "# comment-lint:ignore\n\
               # The pool boots; we go in order to warm it.\n\
               x = 1\n\
               # In order to check the second block.\n\
               y = 2\n";
    let d = lint_rb(src, &["STE006", "STE007"]);
    assert_eq!(d.len(), 1, "only the block after the directive is exempt");
    assert_eq!(d[0].line, 4);
}

#[test]
fn c_style_block_comment_is_unwrapped_and_linted() {
    let src = "/* In order to boot\n * we utilize the pool.\n */\nconst x = 1\n";
    let d = lint_js(src, &["STE006"]);
    assert_eq!(d.len(), 2, "both phrases found: {:?}", msgs(&d));
    assert!(d.iter().all(|f| f.line == 1), "multi-line comment maps to its start");
}

#[test]
fn ruby_equals_begin_comment_is_unwrapped_and_linted() {
    let src = "=begin\nIn order to boot we utilize the pool.\n=end\nx = 1\n";
    let d = lint_rb(src, &["STE006"]);
    assert_eq!(d.len(), 2, "{:?}", msgs(&d));
}

#[test]
fn procedural_sentence_gets_the_tighter_limit() {
    // Word limits apply in doc position, so both comments sit above a def.
    // 21 words, imperative lead ("Set") -> over the 20-word procedural cap.
    let s = "Set the pool size to ten when the app boots so the first busy request does not wait on a handshake.";
    assert_eq!(s.split_whitespace().count(), 21);
    let src = format!("# {s}\ndef boot\n  x\nend\n");
    let d = lint_rb(&src, &["STE001"]);
    assert_eq!(d.len(), 1, "{:?}", msgs(&d));

    // Same length, descriptive lead -> under the 25-word descriptive cap.
    let s2 = "The pool size is ten when the app boots so the first busy request does not wait on a slow handshake.";
    assert_eq!(s2.split_whitespace().count(), 21);
    let src2 = format!("# {s2}\ndef boot\n  x\nend\n");
    assert!(lint_rb(&src2, &["STE001"]).is_empty());
}

#[test]
fn seven_sentences_trip_ste002() {
    let src = "# One a. Two b. Three c. Four d. Five e. Six f. Seven g.\nx = 1\n";
    let d = lint_rb(src, &["STE002"]);
    assert_eq!(d.len(), 1);
    assert_eq!(d[0].code, "STE002");
}

#[test]
fn abbreviations_do_not_split_sentences() {
    // "e.g." must not create sentence boundaries; two sentences total.
    let src = "# The pool warms early, e.g. at boot. It refills after that.\nx = 1\n";
    let d = lint_rb(src, &["STE002"]);
    assert!(d.is_empty());
}

#[test]
fn mask_preserves_byte_length() {
    let cases = [
        "Call `Widget#find_by_slug` to get the record.",
        "See https://example.com/a?b=c for details.",
        "The snake_case and camelCase and SCREAMING names.",
        "Version 1.2.3 calls foo(bar, baz).",
        "A `sp\u{e9}cial` span with multibyte content.",
        "Foo::Bar.baz chains and {interpolation} too.",
    ];
    for c in cases {
        assert_eq!(mask(c).len(), c.len(), "length invariant broke on: {c}");
    }
}

#[test]
fn all_four_grammars_parse() {
    for (path, comment) in [
        ("a.rb", "# In order to boot.\nx = 1\n"),
        ("a.js", "// In order to boot.\nconst x = 1\n"),
        ("a.ts", "// In order to boot.\nconst x: number = 1\n"),
        ("a.tsx", "// In order to boot.\nconst x = <div/>\n"),
    ] {
        let enabled = vec!["STE006".to_string()];
        let d = lint_source(path, comment, &enabled, &Limits::default());
        assert_eq!(d.len(), 1, "{path} must parse and find the phrase");
    }
}

fn msgs(d: &[Diagnostic]) -> Vec<String> {
    d.iter().map(|f| format!("{}:{} {} {}", f.line, f.column, f.code, f.message)).collect()
}
