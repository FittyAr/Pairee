//! Built-in languages whose regions end with text captured by the start
//! (heredocs, raw strings, long brackets) or may start only after certain
//! text (regular expression literals).

use super::check_file;
use crate::syntax::SyntaxKind::{
    Comment, Constant, Function, Keyword, Operator, String as Str, TypeName,
};

#[test]
fn shell_heredocs_end_at_their_own_word() {
    let lines = [
        "cat <<EOF > out",
        "Hello $USER",
        "EOFX is not the end",
        "EOF",
        "echo done",
        "cat <<-'END'",
        "\tliteral",
        "\tEND",
        "grep a <<< \"$x\"",
        "echo after",
    ];
    check_file(
        "run.sh",
        &lines,
        &[
            (0, "<<EOF", Some(Str)),
            (1, "Hello", Some(Str)),
            (1, "$USER", Some(Constant)),
            (2, "EOFX", Some(Str)),
            (3, "EOF", Some(Str)),
            (4, "echo", Some(Function)),
            (6, "literal", Some(Str)),
            (7, "END", Some(Str)),
            (8, "<<<", Some(Operator)),
            (8, "\"$x\"", Some(Str)),
            (9, "echo", Some(Function)),
        ],
    );
}

#[test]
fn php_heredocs_and_nowdocs() {
    let lines = [
        "<?php",
        "$s = <<<EOT",
        "Dear $name",
        "  EOT;",
        "$t = <<<'RAW'",
        "EOT $x",
        "RAW;",
        "echo $s;",
    ];
    check_file(
        "mail.php",
        &lines,
        &[
            (1, "$s", Some(TypeName)),
            (2, "Dear", Some(Str)),
            (2, "$name", Some(Constant)),
            (3, "EOT", Some(Str)),
            (4, "$t", Some(TypeName)),
            (5, "EOT", Some(Str)),
            (6, "RAW", Some(Str)),
            (7, "echo", Some(Keyword)),
        ],
    );
}

#[test]
fn rust_raw_strings_need_as_many_hashes() {
    let lines = [
        r###"let s = r##"a "# still"##; let t = 1;"###,
        r#"let u = r#"open"#,
        r##"still "inside"##,
        r##"end"# fn"##,
    ];
    check_file(
        "lib.rs",
        &lines,
        &[
            (0, "still", Some(Str)),
            (0, "let t", Some(Keyword)),
            (2, "inside", Some(Str)),
            (3, "end", Some(Str)),
            (3, "fn", Some(Keyword)),
        ],
    );
}

#[test]
fn lua_long_brackets_match_their_level() {
    let lines = [
        "s = [==[ a ]] b ]=] c ]==] x = nil",
        "--[=[ comment ]]",
        "still ]=] local y",
    ];
    check_file(
        "init.lua",
        &lines,
        &[
            (0, "b ", Some(Str)),
            (0, "c ", Some(Str)),
            (0, "nil", Some(Constant)),
            (1, "comment", Some(Comment)),
            (2, "still", Some(Comment)),
            (2, "local", Some(Keyword)),
        ],
    );
}

#[test]
fn cpp_raw_strings_end_at_their_delimiter() {
    let lines = [
        r#"auto s = R"xy(a )" b)xy"; return 0;"#,
        r#"auto t = R"(c)";"#,
    ];
    check_file(
        "main.cpp",
        &lines,
        &[
            (0, "b)", Some(Str)),
            (0, "return", Some(Keyword)),
            (1, "c)", Some(Str)),
        ],
    );
}

#[test]
fn javascript_tells_regular_expressions_from_division() {
    let lines = [
        "let a = b / c / d;",
        "let r = /ab+c/gi.test(s);",
        "if (/[/]x/.test(s)) return /y\\/z/;",
        "x = a[0] / 2 // half",
        "f(1) / g(2)",
        "/^start/.exec(t)",
    ];
    check_file(
        "app.js",
        &lines,
        &[
            (0, "c ", None),
            (0, "/ c", Some(Operator)),
            (1, "/ab", Some(Str)),
            (1, "gi", Some(Str)),
            (1, "test", Some(Function)),
            (2, "[/]x", Some(Str)),
            (2, "return", Some(Keyword)),
            (2, "\\/", Some(Constant)),
            (2, "z/", Some(Str)),
            (3, "2", Some(Constant)),
            (3, "// half", Some(Comment)),
            (4, "g", Some(Function)),
            (5, "^start", Some(Str)),
        ],
    );
    let ts = ["const n = total / count / 2;", "const m = s.split(/,/);"];
    check_file("a.ts", &ts, &[(0, "count", None), (1, "/,/", Some(Str))]);
}

#[test]
fn ruby_regular_expressions_and_heredocs() {
    let lines = [
        "if line =~ /^#\\s+(\\w+)/ then",
        "avg = total / count / 2",
        "text = <<~EOS",
        "  Hi #{name}",
        "  EOS",
        "puts 1",
    ];
    check_file(
        "a.rb",
        &lines,
        &[
            (0, "^#", Some(Str)),
            (0, "then", Some(Keyword)),
            (1, "count", None),
            (3, "Hi", Some(Str)),
            (3, "#{name}", Some(Constant)),
            (5, "1", Some(Constant)),
        ],
    );
}
