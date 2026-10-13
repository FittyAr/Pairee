# Syntax definitions

One TOML file per language, embedded in the Pairee binary (`src/syntax/builtin.rs`)
and used by the syntax highlighting of the built-in editor and viewer. The format is
described in the help (`help/en/configuration_details.md`, "Syntax highlighting
definitions"). Users can add languages, or replace these ones, by dropping files in
the same format into the `syntax/` folder of the Pairee configuration directory.

## Adding a language

1. Write `<language>.toml` here (see any file for an example).
2. Add its stem to `FILES` in `src/syntax/builtin.rs`.
3. Add a sample to the language tests in `src/syntax/tests/languages.rs`.

## Credits

The format and the approach (regular-expression rules plus start/end regions,
highlighted line by line) follow the syntax files of the
[micro editor](https://github.com/zyedidia/micro/tree/master/runtime/syntax), whose
keyword lists and patterns served as a reference for several of these definitions.
micro is distributed under the MIT License:

> Copyright (c) 2016-2020: Zachary Yedidia, et al.
>
> Permission is hereby granted, free of charge, to any person obtaining a copy of this
> software and associated documentation files (the "Software"), to deal in the
> Software without restriction, including without limitation the rights to use, copy,
> modify, merge, publish, distribute, sublicense, and/or sell copies of the Software,
> and to permit persons to whom the Software is furnished to do so, subject to the
> following conditions:
>
> The above copyright notice and this permission notice shall be included in all
> copies or substantial portions of the Software.
>
> THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED,
> INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A
> PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT
> HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF
> CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE
> OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.
