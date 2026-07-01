import os
import re

en_toml_path = r"lang/en.toml"
es_toml_path = r"lang/es.toml"
src_dir = r"src"

def get_translation_keys(path):
    keys = []
    in_translations = False
    with open(path, 'r', encoding='utf-8') as f:
        for line in f:
            line = line.strip()
            if line == "[translations]":
                in_translations = True
                continue
            if in_translations and '=' in line:
                key = line.split('=', 1)[0].strip()
                if key:
                    keys.append(key)
    return keys

def clean_toml(path, unused_keys):
    lines = []
    in_translations = False
    with open(path, 'r', encoding='utf-8') as f:
        for line in f:
            stripped = line.strip()
            if stripped == "[translations]":
                in_translations = True
                lines.append(line)
                continue
            if in_translations and '=' in stripped:
                key = stripped.split('=', 1)[0].strip()
                if key in unused_keys:
                    continue
            lines.append(line)
    
    with open(path, 'w', encoding='utf-8') as f:
        f.writelines(lines)

# 1. Get all keys
keys = get_translation_keys(en_toml_path)
print(f"Total keys found in en.toml: {len(keys)}")

# 2. Find all .rs files
rs_files = []
for root, _, files in os.walk(src_dir):
    for file in files:
        if file.endswith('.rs'):
            rs_files.append(os.path.join(root, file))

print(f"Scanning {len(rs_files)} Rust source files...")

# Read content of all .rs files
rs_contents = []
for path in rs_files:
    try:
        with open(path, 'r', encoding='utf-8') as f:
            rs_contents.append(f.read())
    except Exception as e:
        print(f"Error reading {path}: {e}")

# 3. Check usage of each key
unused_keys = set()
for key in keys:
    found = False
    for content in rs_contents:
        if key in content:
            found = True
            break
    if not found:
        unused_keys.add(key)

print(f"Found {len(unused_keys)} unused keys.")
if unused_keys:
    print("Unused keys to be removed:")
    for k in sorted(unused_keys):
        print(f"  - {k}")
    
    # 4. Clean both files
    print("Cleaning en.toml...")
    clean_toml(en_toml_path, unused_keys)
    print("Cleaning es.toml...")
    clean_toml(es_toml_path, unused_keys)
    print("Cleanup complete!")
else:
    print("No unused keys found.")
