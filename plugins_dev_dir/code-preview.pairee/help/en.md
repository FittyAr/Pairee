# code-preview.pairee — Syntax-highlighted code preview

Press <F8> to render the highlighted preview of the current code
file. The preview uses the embedded `syntect` syntax set (no
external binary required).

## Supported languages

Rust, Python, JavaScript/TypeScript, Go, Java, C/C++, TOML, JSON,
XML/HTML, CSS, Shell/Bash, YAML, Markdown, SQL, Lua, Swift, Kotlin,
Dart, Elixir, Ruby, PHP, Perl, Vim script.

## How it works

- The plugin hooks `peek(job)` so it intercepts the standard
  preview path.
- For code files it calls `pairee.preview_code({ path })` which
  returns a `ui.Text` userdata with per-token `ui.Span`s.
- The text is sent through `pairee.preview_widget(opts, text)` to
  the preview pane.
