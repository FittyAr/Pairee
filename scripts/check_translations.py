#!/usr/bin/env python3
"""Checks the translation files (lang/*.toml) and the help pages (help/<lang>/).

Run by `cargo xtask lint` (pre-push hook) and the CI policy job; it never
rewrites files. Needs Python 3.11+ (tomllib). Fails when:

* a language file is not valid TOML, or has top-level keys other than
  `language_name` and `[translations]` (`t()` only reads `[translations]`);
* lang/es.toml and lang/en.toml do not share the same keys, or a value
  does not keep the `{}` / `{name}` placeholders of its English one;
* the source passes a key to `t("...")` / `tr("...")` that en.toml lacks;
* help/en and help/es do not hold the same pages, or a page has no
  `help_title_<page>` key (the help index would show its file name).

`--unused` also lists the keys no source file names, either literally or
through a `format!("prefix_{...}")` key; review them before deleting any.
"""

from pathlib import Path
import re
import sys

try:
    import tomllib
except ImportError:
    sys.exit("check_translations.py needs Python 3.11 or newer (tomllib).")

ROOT = Path(__file__).resolve().parents[1]
LANGS = ("en", "es")
TOP_LEVEL = {"language_name", "translations"}
PLACEHOLDER = re.compile(r"\{[A-Za-z0-9_]*\}")
# `t("key")` / `tr("key")` with a literal key, and the helpers that take
# the key first: `t_fill("key", ..)`, `localized("key", ..)`.
LOOKUP = re.compile(
    r'\b(?:(?:t|tr)\(\s*"([a-z][a-z0-9_.]*)"\s*\)|(?:t_fill|localized)\(\s*"([a-z][a-z0-9_.]*)"\s*,)'
)
# Keys built at run time: `format!("action_{}", id)` covers `action_*`.
KEY_PREFIX = re.compile(r'format!\(\s*"([a-z][a-z0-9_]*_)\{')
LITERAL = re.compile(r'"([a-z][a-z0-9_.]*)"')
# Source files that name keys on purpose without defining them.
TEST_ONLY_KEYS = {"unknown_missing_key_xyz"}


def load_language(code: str, errors: list[str]) -> dict[str, str]:
    path = ROOT / "lang" / f"{code}.toml"
    try:
        data = tomllib.loads(path.read_text(encoding="utf-8"))
    except (OSError, tomllib.TOMLDecodeError) as e:
        errors.append(f"{path.name}: {e}")
        return {}
    extra = sorted(set(data) - TOP_LEVEL)
    if extra:
        errors.append(f"{path.name}: keys outside [translations] are never read: {', '.join(extra)}")
    translations = data.get("translations", {})
    return {k: v for k, v in translations.items() if isinstance(v, str)}


def source_text() -> str:
    return "\n".join(
        p.read_text(encoding="utf-8", errors="replace") for p in sorted((ROOT / "src").rglob("*.rs"))
    )


def check_keys(en: dict[str, str], es: dict[str, str], errors: list[str]) -> None:
    for name, keys in (("es.toml", set(en) - set(es)), ("en.toml", set(es) - set(en))):
        for key in sorted(keys):
            errors.append(f"{name}: missing key {key}")
    for key in sorted(set(en) & set(es)):
        want, got = sorted(PLACEHOLDER.findall(en[key])), sorted(PLACEHOLDER.findall(es[key]))
        if want != got:
            errors.append(f"es.toml: {key} has placeholders {got}, en.toml {want}")


def check_lookups(en: dict[str, str], source: str, errors: list[str]) -> None:
    used = {plain or first for plain, first in LOOKUP.findall(source)}
    for key in sorted(used - set(en) - TEST_ONLY_KEYS):
        errors.append(f"en.toml: missing key {key} (used by t()/tr() in src/)")


def check_help(en: dict[str, str], errors: list[str]) -> None:
    pages = {code: {p.stem for p in (ROOT / "help" / code).glob("*.md")} for code in LANGS}
    for code in LANGS:
        for other in LANGS:
            for page in sorted(pages[other] - pages[code]):
                errors.append(f"help/{code}: missing {page}.md (present in help/{other})")
    for page in sorted(pages["en"]):
        if f"help_title_{page}" not in en:
            errors.append(f"en.toml: missing key help_title_{page} (help/en/{page}.md)")


def unused_keys(en: dict[str, str], source: str) -> list[str]:
    literals = set(LITERAL.findall(source))
    prefixes = tuple(set(KEY_PREFIX.findall(source)))
    return sorted(k for k in en if k not in literals and not k.startswith(prefixes))


def main() -> int:
    errors: list[str] = []
    en = load_language("en", errors)
    es = load_language("es", errors)
    source = source_text()
    check_keys(en, es, errors)
    check_lookups(en, source, errors)
    check_help(en, errors)

    print(f"en.toml: {len(en)} keys, es.toml: {len(es)} keys")
    if "--unused" in sys.argv[1:]:
        unused = unused_keys(en, source)
        print(f"Keys no source file names ({len(unused)}):")
        for key in unused:
            print(f"  {key}")
    if errors:
        for error in errors:
            print(f"error: {error}")
        print("See docs/i18n.md.")
        return 1
    print("OK: translations and help pages are consistent.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
