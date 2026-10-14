//! Built-in languages embedding others: HTML with JavaScript, CSS and PHP,
//! PHP files (HTML hosting PHP) and Markdown code blocks.

use super::check_file;
use crate::syntax::SyntaxKind::{
    Comment, Function, Keyword, Operator, String as Str, Tag, TypeName,
};

#[test]
fn html_embeds_javascript_and_css() {
    let lines = [
        "<html><script type=\"module\">",
        "const s = \"</div>\"; if (x < 2) {}",
        "</script><style>",
        "a { color: red; } /* c */",
        "</style><p class=\"x\">t</p>",
        "<script>let q = '</script>'; <b>after</b>",
    ];
    check_file(
        "index.html",
        &lines,
        &[
            (0, "<html", Some(Tag)),
            (0, "<script", Some(Tag)),
            (1, "const", Some(Keyword)),
            (1, "\"</div>\"", Some(Str)),
            (1, "if", Some(Keyword)),
            (1, "< 2", Some(Operator)),
            (2, "</script>", Some(Tag)),
            (2, "<style>", Some(Tag)),
            (3, "color", Some(TypeName)),
            (3, "/* c", Some(Comment)),
            (4, "</style>", Some(Tag)),
            (4, "class", Some(Function)),
            (5, "let", Some(Keyword)),
            (5, "'</", Some(Str)),
            (5, "</script>", Some(Tag)),
            (5, "<b", Some(Tag)),
            (5, "after", None),
        ],
    );
}

#[test]
fn php_files_are_html_with_php_inside() {
    let lines = [
        "<p>Hi</p>",
        "<?php if ($a) { // note ?>",
        "<b>yes</b>",
        "<?= $name ?> <i>x</i>",
    ];
    check_file(
        "page.php",
        &lines,
        &[
            (0, "<p", Some(Tag)),
            (0, "Hi", None),
            (1, "<?php", Some(Tag)),
            (1, "if", Some(Keyword)),
            (1, "$a", Some(TypeName)),
            (1, "// note", Some(Comment)),
            (1, "?>", Some(Tag)),
            (2, "<b", Some(Tag)),
            (2, "yes", None),
            (3, "$name", Some(TypeName)),
            (3, "<i", Some(Tag)),
        ],
    );
}

#[test]
fn markdown_code_blocks_use_their_language() {
    let lines = [
        "````rust",
        "fn main() {}",
        "```",
        "let x = \"s\";",
        "````",
        "after *em*",
        "~~~ unknown-lang",
        "fn plain",
        "~~~",
        "```js",
        "const r = /a+/g;",
        "```",
        "```",
        "if no language",
        "```",
    ];
    check_file(
        "README.md",
        &lines,
        &[
            (0, "````rust", Some(Str)),
            (1, "fn", Some(Keyword)),
            (1, "main", Some(Function)),
            (2, "```", None),
            (3, "let", Some(Keyword)),
            (3, "\"s\"", Some(Str)),
            (4, "````", Some(Str)),
            (5, "after", None),
            (5, "*em*", Some(TypeName)),
            (6, "~~~", Some(Str)),
            (7, "fn", None),
            (8, "~~~", Some(Str)),
            (10, "const", Some(Keyword)),
            (10, "/a+/g", Some(Str)),
            (11, "```", Some(Str)),
            (13, "if", None),
        ],
    );
}
