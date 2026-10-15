# Screenshot tooling

The screenshots in `assets/screens` are real captures of Pairee: the actual `pairee` binary runs in a
pseudo-terminal inside a throwaway Linux container, browsing the home folder of a **fictional** user
("alex", host "devbox"). Every file in that folder is generated here (names, text, numbers and the
pictures, drawn with Pillow); nothing comes from a real machine, and no real user name or path appears.

| File | What it does |
|------|--------------|
| `make_tree.py` | Writes the fictional home folder (projects, documents, photos, archives) with fixed dates. |
| `setup.sh` | Runs in the container: creates the user, copies the folder, adds a git history and large sparse files, writes Pairee's `config.toml`, builds `capture` and replays the scenario. |
| `scenario.txt` | The keys pressed and the screens saved, in gallery order. |
| `capture/` | Small Rust tool: starts `pairee` in a pseudo-terminal (`portable-pty`), sends the keys and saves the terminal screen (`vt100`) as JSON. |
| `render.py` | Draws each JSON screen to PNG like a terminal (Cascadia Mono; box and block characters as shapes). |
| `publish.py` | Makes the WebP slides and thumbnails, the favicons, the social card and the brand art. |

Requirements: Docker (any Linux image with Rust, e.g. `rust:latest`), Python 3 with Pillow on the
host, a checkout of `master` and this branch as a worktree next to it (`../Pairee-site`). The renderer
uses Cascadia Mono, which ships with Windows 11; elsewhere pass `--font` with a monospace TTF.

## Steps

From the root of the `master` checkout. `OUT` is any scratch folder.

```bash
OUT=/tmp/pairee-showcase
T=../Pairee-site/tools/showcase

# 1. A Linux build of pairee (debug is enough), kept in a Docker volume.
docker run --rm -v "$PWD:/src" -v pairee-linux-target:/target -e CARGO_TARGET_DIR=/target \
    -w /src rust:latest cargo build

# 2. The fictional home folder.
python $T/make_tree.py $OUT/tree

# 3. Replay the scenario. /src gives Pairee its help pages and translations.
docker run --rm --hostname devbox \
    -v "$OUT/tree:/tree:ro" -v "$PWD/$T:/t:ro" -v "$PWD:/src:ro" -v "$OUT/json:/out" \
    -v pairee-linux-target:/target:ro -v pairee-showcase-target:/captarget \
    rust:latest sh -c "ln -s /target/debug/pairee /pairee && sh /t/setup.sh"

# 4. Render and publish.
python $T/render.py $OUT/json $OUT/png
python $T/publish.py $OUT/png ../Pairee-site/assets .
```

`COLS`, `ROWS` (default 140 × 40) and `SCENARIO` can be passed to the container with `-e`.

Check every PNG before publishing: if the interface changed, adjust the key presses in
`scenario.txt` (the comments say what each screen should show). Keys after a Ctrl or Alt shortcut
are followed by `Down Up` because the key bar shows that modifier's layer until the next key.
