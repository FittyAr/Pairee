#!/bin/sh
# Runs inside the throwaway container (see README.md): creates the fictional
# user "alex", copies the tree from make_tree.py, adds a git history, builds
# the capture tool and replays the scenario.
#
#   /tree     output of make_tree.py      /t       this folder
#   /pairee   the pairee binary to film   /out     screen dumps (JSON)
set -eu

id alex > /dev/null 2>&1 || useradd -m -u 1000 -s /bin/bash alex
H=/home/alex
cp -a /tree/alex/. "$H/"
# Sizes worth showing, without the bytes (sparse files).
truncate -s 5900M "$H/Downloads/ubuntu-26.04-desktop-amd64.iso"
truncate -s 734M "$H/Videos/garden-timelapse-2026.mp4"
touch -d "2026-09-20 18:40" "$H/Downloads/ubuntu-26.04-desktop-amd64.iso"
touch -d "2026-08-30 07:15" "$H/Videos/garden-timelapse-2026.mp4"
for dir in Downloads Videos; do
    touch -r "$(ls -t "$H/$dir"/* | head -n 1)" "$H/$dir"
done
find "$H" -type d -exec chmod 755 {} +
find "$H" -type f -exec chmod 644 {} +
chmod 755 "$H/Downloads/install-tools.sh"

# A short git history for the git screens, with uncommitted changes.
REPO="$H/Projects/aurora-web"
mv "$REPO/src/routes.rs" /tmp/routes.rs
cd "$REPO"
git init -q -b main
git config user.name "Alex Rivera"
git config user.email "alex.rivera@example.com"
commit() {
    GIT_AUTHOR_DATE="$1" GIT_COMMITTER_DATE="$1" git commit -q -m "$2"
}
git add Cargo.toml LICENSE .gitignore src/main.rs src/config.rs && commit "2026-08-02T10:12:00" "Start the booking service"
git add src/db.rs migrations && commit "2026-08-09T16:40:00" "Store plots in SQLite"
printf 'use axum::Router;\n\npub fn router(pool: sqlx::SqlitePool) -> Router {\n    Router::new().with_state(pool)\n}\n' > src/routes.rs
git add src/routes.rs && commit "2026-08-21T09:05:00" "Add the HTTP routes"
git add tests docs && commit "2026-09-12T18:22:00" "Document the architecture and test the API"
git add assets rustfmt.toml Cargo.lock && commit "2026-09-19T15:02:00" "Add the logo and pin the dependencies"
git add README.md CHANGELOG.md && commit "2026-09-26T11:30:00" "Explain how to run it"
git tag v0.4.2
# Work in progress: one change staged, two not, one new file.
cp /tmp/routes.rs src/routes.rs
touch -d "2026-09-28 09:20" src/routes.rs
printf '\n## Reminders\n\nMembers get an e-mail the evening before their turn.\n' >> README.md
touch -d "2026-09-28 10:05" README.md
git add README.md
sed -i 's/version = "0.4.2"/version = "0.5.0"/' Cargo.toml
touch -d "2026-09-28 10:12" Cargo.toml
printf '# Seeding\n\nLoad the sample plots with `cargo run -- seed`.\n' > docs/seeding.md
touch -d "2026-09-27 21:05" docs/seeding.md
# Git touched the folders: give them the dates of their newest file.
for dir in src docs tests migrations assets .; do
    touch -r "$(ls -t "$dir"/* | head -n 1)" "$dir"
done
cd /

CONFIG="$H/.config/FittyAr/Pairee"
mkdir -p "$CONFIG"
cat > "$CONFIG/config.toml" << 'EOF'
onboarding_completed = true
auto_update_check = false
plugins_enabled = false
interface_clock = false
language = "English"
EOF
chown -R alex:alex "$H"

# The capture tool, built once into /captarget.
rm -rf /tmp/capture
cp -r /t/capture /tmp/capture
CARGO_TARGET_DIR=/captarget cargo build -q --release --manifest-path /tmp/capture/Cargo.toml
cp /tmp/capture/Cargo.lock /out/capture-Cargo.lock
mkdir -p /out
chown alex /out
runuser -u alex -- env -i HOME="$H" USER=alex LOGNAME=alex SHELL=/bin/bash \
    PATH=/usr/local/bin:/usr/bin:/bin LANG=C.UTF-8 \
    /captarget/release/pairee-showcase-capture /pairee "${COLS:-140}" "${ROWS:-40}" \
    "/t/${SCENARIO:-scenario.txt}" /out
