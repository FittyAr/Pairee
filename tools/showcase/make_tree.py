"""Create the fictional home folder the screenshots browse (user "alex").

Usage: python make_tree.py <out-dir>

Everything here is invented: names, text, numbers and pictures (drawn with
Pillow). Nothing is copied from a real machine. `setup.sh` copies the result
to /home/alex inside the container and adds what needs Linux (a git history,
a large sparse file, permissions).
"""

import io
import math
import os
import random
import sys
import tarfile
import time
import zipfile
from pathlib import Path

from PIL import Image, ImageDraw

root = Path(sys.argv[1]) / "alex"
rng = random.Random(42)
# Fixed dates (2026), so every run produces the same listing.
BASE = time.mktime((2026, 9, 28, 10, 0, 0, 0, 0, -1))


def write(rel, content, days_ago=None):
    path = root / rel
    path.parent.mkdir(parents=True, exist_ok=True)
    data = content.encode() if isinstance(content, str) else content
    path.write_bytes(data)
    stamp(path, days_ago)
    return path


def stamp(path, days_ago=None):
    days = rng.uniform(0.2, 60) if days_ago is None else days_ago + rng.uniform(-0.3, 0.3)
    t = BASE - days * 86400
    os.utime(path, (t, t))


# ── Projects/aurora-web: a small Rust web service ────────────────────────────

write("Projects/aurora-web/Cargo.toml", """[package]
name = "aurora-web"
version = "0.4.2"
edition = "2024"
description = "A tiny booking service for community gardens"
license = "MIT"

[dependencies]
axum = "0.8"
serde = { version = "1", features = ["derive"] }
sqlx = { version = "0.8", features = ["sqlite", "runtime-tokio"] }
tokio = { version = "1", features = ["full"] }
tracing = "0.1"
""", 3)
write("Projects/aurora-web/README.md", """# Aurora

A tiny booking service for community gardens: members reserve a plot, the
coordinator sees the calendar, everybody gets a reminder before their turn.

## Running

```sh
cargo run -- --db gardens.sqlite --port 8080
```

## Layout

- `src/routes.rs`: HTTP handlers
- `src/db.rs`: queries and migrations
- `docs/architecture.md`: how the pieces fit
""", 2)
write("Projects/aurora-web/LICENSE", "MIT License\n\nCopyright (c) 2026 Alex Rivera\n\nPermission is hereby granted, free of charge, to any person obtaining a copy\nof this software...\n", 40)
write("Projects/aurora-web/.gitignore", "/target\n*.sqlite\n.env\n", 40)
write("Projects/aurora-web/src/main.rs", """use std::net::SocketAddr;

mod config;
mod db;
mod routes;

/// Starts the booking service.
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    let config = config::Config::from_args()?;
    let pool = db::connect(&config.database).await?;
    db::migrate(&pool).await?;

    let app = routes::router(pool);
    let addr = SocketAddr::from(([127, 0, 0, 1], config.port));
    tracing::info!("listening on http://{addr}");
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}
""", 1.2)
write("Projects/aurora-web/src/routes.rs", """use axum::{Json, Router, extract::{Path, State}, routing::{get, post}};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

#[derive(Serialize)]
pub struct Plot {
    id: i64,
    name: String,
    size_m2: f32,
    free: bool,
}

#[derive(Deserialize)]
pub struct Booking {
    plot: i64,
    member: String,
    week: u32,
}

pub fn router(pool: SqlitePool) -> Router {
    Router::new()
        .route("/plots", get(list_plots))
        .route("/plots/{id}", get(plot))
        .route("/bookings", post(book))
        .with_state(pool)
}

async fn list_plots(State(pool): State<SqlitePool>) -> Json<Vec<Plot>> {
    let plots = crate::db::plots(&pool).await.unwrap_or_default();
    Json(plots)
}

async fn plot(State(pool): State<SqlitePool>, Path(id): Path<i64>) -> Option<Json<Plot>> {
    crate::db::plot(&pool, id).await.ok().flatten().map(Json)
}

async fn book(State(pool): State<SqlitePool>, Json(b): Json<Booking>) -> &'static str {
    match crate::db::book(&pool, b.plot, &b.member, b.week).await {
        Ok(()) => "booked",
        Err(_) => "that week is taken",
    }
}
""", 0.3)
write("Projects/aurora-web/src/db.rs", """use sqlx::SqlitePool;

pub async fn connect(url: &str) -> sqlx::Result<SqlitePool> {
    SqlitePool::connect(url).await
}

pub async fn migrate(pool: &SqlitePool) -> sqlx::Result<()> {
    sqlx::query(include_str!("../migrations/001_plots.sql")).execute(pool).await?;
    Ok(())
}
""", 5)
write("Projects/aurora-web/src/config.rs", """/// Command-line options.
pub struct Config {
    pub database: String,
    pub port: u16,
}
""", 12)
write("Projects/aurora-web/migrations/001_plots.sql", "CREATE TABLE IF NOT EXISTS plots (\n  id INTEGER PRIMARY KEY,\n  name TEXT NOT NULL,\n  size_m2 REAL NOT NULL\n);\n", 12)
write("Projects/aurora-web/tests/api.rs", "#[tokio::test]\nasync fn lists_the_plots() {\n    // ...\n}\n", 4)
write("Projects/aurora-web/docs/architecture.md", "# Architecture\n\nOne binary, one SQLite file. Requests go through `routes`, which only\ntalks to `db`.\n", 9)
write("Projects/aurora-web/rustfmt.toml", "max_width = 100\n", 30)
write("Projects/aurora-web/Cargo.lock", "# This file is automatically @generated by Cargo.\nversion = 4\n\n"
      + "\n".join(f'[[package]]\nname = "dep-{i}"\nversion = "1.{i}.0"\n' for i in range(140)), 3)
write("Projects/aurora-web/CHANGELOG.md", "# Changelog\n\n## 0.4.2\n- Weekly reminders\n\n## 0.4.0\n- Plot calendar\n", 2)
write("Projects/aurora-web/assets/logo.svg", '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 64 64"><circle cx="32" cy="32" r="28" fill="#4caf50"/></svg>\n', 20)

for name, files in {
    "terra-cli": {"main.go": "package main\n\nfunc main() {\n\tprintln(\"terra: soil moisture logger\")\n}\n", "go.mod": "module example.com/terra\n\ngo 1.25\n", "README.md": "# terra\n\nLogs soil moisture from a garden sensor.\n"},
    "notes-sync": {"sync.py": "\"\"\"Mirror a notes folder to a backup drive.\"\"\"\n\nimport shutil\n", "requirements.txt": "watchdog==6.0.0\n", "README.md": "# notes-sync\n"},
    "website": {"index.html": "<!doctype html>\n<title>Alex Rivera</title>\n<h1>Hi, I grow vegetables and write code.</h1>\n", "styles.css": "body { font-family: system-ui; }\n"},
}.items():
    for f, c in files.items():
        write(f"Projects/{name}/{f}", c)

# ── Documents ────────────────────────────────────────────────────────────────

rows = ["month,rent,groceries,transport,garden,savings"]
for m in ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep"]:
    rows.append(f"{m},950,{rng.randint(310, 420)},{rng.randint(60, 110)},{rng.randint(10, 80)},{rng.randint(200, 450)}")
write("Documents/Budget 2026.csv", "\n".join(rows) + "\n", 1.5)
write("Documents/Meeting notes - Q3 planning.md", """# Q3 planning — garden cooperative

**Present:** Alex, Sam, Priya, Jordan

## Decisions
- Open six new plots by the east fence.
- Rain barrels: order four more before October.
- Move the tool shed key to the lockbox.

## Next steps
- [x] Alex: update the booking site with the new plots
- [ ] Priya: ask the city about the water meter
- [ ] Sam: workshop on composting, second Saturday
""", 6)
write("Documents/Reading list.txt", "The Overstory\nBraiding Sweetgrass\nThe Rust Programming Language\nA Pattern Language\n", 30)
write("Documents/Invoice-0042.pdf", b"%PDF-1.4\n% sample invoice, not a real document\n" + bytes(rng.getrandbits(8) for _ in range(48_000)), 14)
write("Documents/Recipes/Sourdough.md", "# Sourdough\n\n- 500 g flour\n- 350 g water\n- 100 g starter\n- 10 g salt\n", 25)
write("Documents/Recipes/Pasta e fagioli.md", "# Pasta e fagioli\n\nBeans, pasta, rosemary, patience.\n", 33)
write("Documents/Recipes/Tomato chutney.md", "# Tomato chutney\n\nFor the August glut.\n", 48)
write("Documents/Taxes/2025-summary.txt", "Sample figures only.\nIncome: 0\nDeductions: 0\n", 120)
write("Documents/Volunteer roster.csv", "name,day,task\nSam,Sat,watering\nPriya,Sun,compost\nJordan,Wed,tools\n", 4)
write("Documents/Seed inventory.md", "# Seeds\n\n| Crop | Packets |\n|---|---|\n| Beans | 4 |\n| Kale | 2 |\n", 9)
write("Documents/Letter to the city council.md", "Dear council,\n\nThe garden cooperative would like to ask...\n", 21)
write("Documents/Workshop poster.svg", '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 140"><rect width="100" height="140" fill="#2e7d32"/></svg>\n', 11)
write("Documents/Harvest log 2026.csv", "date,crop,kg\n2026-07-02,zucchini,3.4\n2026-07-19,tomatoes,6.1\n2026-08-11,beans,2.2\n", 2.5)
write("Documents/Insurance - garden shed.pdf", b"%PDF-1.4\n% sample, not a real document\n" + bytes(rng.getrandbits(8) for _ in range(120_000)), 75)
write("Documents/Garden plan 2027.txt", "Bed 1: beans, squash\nBed 2: tomatoes, basil\nBed 3: garlic (plant in Oct)\n", 0.8)

# ── Photos: simple drawn scenes ──────────────────────────────────────────────


def scene(seed, w=1600, h=1067):
    r = random.Random(seed)
    sky = [(r.randint(120, 200), r.randint(170, 220), 255), (255, r.randint(190, 230), r.randint(150, 200))]
    img = Image.new("RGB", (w, h))
    d = ImageDraw.Draw(img)
    for y in range(h):
        t = y / h
        d.line([(0, y), (w, y)], fill=tuple(int(sky[0][i] + (sky[1][i] - sky[0][i]) * t) for i in range(3)))
    d.ellipse([w * 0.7, h * 0.15, w * 0.7 + 160, h * 0.15 + 160], fill=(255, 236, 180))
    for layer in range(3):
        base = h * (0.55 + layer * 0.12)
        color = (40 + layer * 20, 110 - layer * 15 + r.randint(0, 30), 60 + layer * 10)
        pts = [(0, h)]
        for x in range(0, w + 40, 40):
            pts.append((x, base - 80 * math.sin(x / (180 + layer * 60) + seed) - r.randint(0, 25)))
        pts.append((w, h))
        d.polygon(pts, fill=color)
    return img


for i in range(8):
    out = io.BytesIO()
    scene(i).save(out, "JPEG", quality=82)
    write(f"Photos/Lisbon 2026/IMG_{412 + i:04d}.jpg", out.getvalue(), 18 - i * 0.1)
for i in range(3):
    out = io.BytesIO()
    scene(20 + i, 1200, 800).save(out, "JPEG", quality=80)
    write(f"Photos/Hiking/trail-{i + 1:02d}.jpg", out.getvalue(), 70 + i)

# ── Downloads: archives and an installer script ──────────────────────────────

buf = io.BytesIO()
with zipfile.ZipFile(buf, "w", zipfile.ZIP_DEFLATED) as z:
    for name in ["README.txt", "fonts/GardenSans-Regular.otf", "fonts/GardenSans-Bold.otf", "LICENSE-OFL.txt"]:
        z.writestr(zipfile.ZipInfo(name, date_time=(2026, 9, 26, 6, 50, 0)), ("sample " * 400 + name) if name.endswith("txt") else bytes(rng.getrandbits(8) for _ in range(30_000)))
write("Downloads/garden-sans-fonts.zip", buf.getvalue(), 2)

buf = io.BytesIO()
with tarfile.open(fileobj=buf, mode="w:gz") as tar:
    for path in sorted((root / "Documents").rglob("*")):
        if path.is_file():
            tar.add(path, arcname=str(path.relative_to(root)).replace(os.sep, "/"))
write("Downloads/documents-backup-2026-09.tar.gz", buf.getvalue(), 16)
write("Downloads/install-tools.sh", "#!/bin/sh\necho 'installing garden tools'\n", 5)
write("Music/Morning playlist.m3u", "#EXTM3U\n# a list of songs, not included\n", 90)
(root / "Videos").mkdir(parents=True, exist_ok=True)
write(".bashrc", "# ~/.bashrc\nexport EDITOR=vi\n", 200)
write(".profile", "# ~/.profile\n", 200)

for d in sorted(root.rglob("*"), reverse=True):
    if d.is_dir():
        stamp(d)
print("tree written to", root)
