#!/usr/bin/env python3
"""Fail if lang/en.toml and lang/es.toml do not share the same translation keys.

Used by CI (`check.yml` policy job) and as the localize-helper verification step.
Does not rewrite files.
"""

from pathlib import Path
import sys

ROOT = Path(__file__).resolve().parents[1]


def translation_keys(path: Path) -> dict[str, str]:
    keys: dict[str, str] = {}
    in_section = False
    text = path.read_text(encoding="utf-8")
    for line in text.splitlines():
        stripped = line.strip()
        if stripped == "[translations]":
            in_section = True
            continue
        if in_section and stripped.startswith("[") and stripped.endswith("]"):
            in_section = False
            continue
        if in_section and "=" in stripped and not stripped.startswith("#"):
            key, _, value = stripped.partition("=")
            keys[key.strip()] = value.strip()
    return keys


def main() -> int:
    en_path = ROOT / "lang" / "en.toml"
    es_path = ROOT / "lang" / "es.toml"
    en = translation_keys(en_path)
    es = translation_keys(es_path)
    only_en = sorted(set(en) - set(es))
    only_es = sorted(set(es) - set(en))
    print(f"en.toml: {len(en)} keys")
    print(f"es.toml: {len(es)} keys")
    if only_en:
        print(f"Missing in es.toml ({len(only_en)}):")
        for k in only_en:
            print(f"  {k}")
    if only_es:
        print(f"Extra in es.toml ({len(only_es)}):")
        for k in only_es:
            print(f"  {k}")
    if only_en or only_es:
        print("Translation key sets must match. See docs/i18n.md.")
        return 1
    print("OK: EN and ES translation keys match.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
