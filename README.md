# Pairee — Official Website

This branch (`gh-pages`) holds the static source of the **Pairee** website, a dual-pane file manager
for the terminal written in Rust.

Live at [pairee.fitty.ar](https://pairee.fitty.ar).

## Contents

- `index.html`: landing page (features, tour, screenshot gallery, install commands and the folders
  Pairee uses).
- `changelog.html`: release history, read at runtime from
  [`docs/CHANGELOG.md`](https://github.com/FittyAr/Pairee/blob/master/docs/CHANGELOG.md) on `master`.
- `plugins.html`: the community plugin catalog, read from the `plugin-registry` branch.
- `css/`, `js/`: styles and scripts; every ES/EN string is in `js/translations.js` and
  `js/common.js` applies them.
- `assets/`: logo (`icon.svg`, from `img/pairee_logo.svg` on `master`), favicons, social card,
  brand art and the screenshots in `assets/screens` (WebP, each with a `-thumb` version).
- `tools/showcase/`: how the screenshots are made — Pairee itself, run in a pseudo-terminal over a
  fictional home folder. See [tools/showcase/README.md](tools/showcase/README.md).
- `llms.txt`, `robots.txt`, `sitemap.xml`, `CNAME`, `.nojekyll`.

## Local development

```bash
git checkout gh-pages
python -m http.server 8000
```

Then open <http://localhost:8000>.

## Pairee source code

The app lives on the [`master` branch](https://github.com/FittyAr/Pairee/tree/master).

## License

Pairee is GPL-3.0; the website files on this branch are under the [MIT License](LICENSE). Manrope and
JetBrains Mono are loaded from Google Fonts (SIL Open Font License 1.1).
