#!/usr/bin/env bash
# bump_version.sh
# Bumps the version in Cargo.toml, commits, tags, and pushes to trigger GitHub CI/CD releases.

set -euo pipefail

# Text formatting
ESC=$(printf '\033')
BOLD="${ESC}[1m"
GREEN="${ESC}[0;32m"
YELLOW="${ESC}[1;33m"
RED="${ESC}[0;31m"
CYAN="${ESC}[0;36m"
RESET="${ESC}[0m"

# Ensure we are in project root
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR/.."

if [[ ! -f "Cargo.toml" ]]; then
    echo -e "${RED}[ERROR]${RESET} Cargo.toml not found."
    exit 1
fi

echo -e "${CYAN}${BOLD}==========================================${RESET}"
echo -e "${CYAN}${BOLD}       Pairee Version Bump & Release      ${RESET}"
echo -e "${CYAN}${BOLD}==========================================${RESET}"

# 1. Check if git has uncommitted changes
if [[ -n "$(git status --porcelain)" ]]; then
    echo -e "${YELLOW}[WARNING] You have uncommitted changes in your repository:${RESET}"
    git status --porcelain
    read -rp "Do you want to proceed anyway? (y/n): " choice
    if [[ "$choice" != "y" && "$choice" != "Y" ]]; then
        echo "Aborted."
        exit 0
    fi
fi

# 2. Get current version from Cargo.toml
current_version=$(grep -m 1 '^version = ' Cargo.toml | sed -E 's/version = "(.*)"/\1/')
echo -e "Current version in Cargo.toml: ${CYAN}$current_version${RESET}"

# Suggest next patch version
IFS='.' read -r major minor patch <<< "$current_version"
next_patch="$major.$minor.$((patch + 1))"

# Prompt user for new version
read -rp "Enter new version [$next_patch]: " new_version
if [[ -z "$new_version" ]]; then
    new_version="$next_patch"
fi

# Validate version format
if [[ ! "$new_version" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
    echo -e "${RED}[ERROR]${RESET} Invalid version format. Must be like 0.1.0"
    exit 1
fi

# 3. Update Cargo.toml and installer.iss
echo -e "Updating Cargo.toml to version ${YELLOW}$new_version${RESET}..."
if [[ "$OSTYPE" == "darwin"* ]]; then
    sed -i '' -E '0,/^version = .*/{s/^version = .*/version = "'"$new_version"'"/;}' Cargo.toml
else
    sed -i -E '0,/^version = .*/s/^version = .*/version = "'"$new_version"'"/' Cargo.toml
fi

if [[ -f "installer.iss" ]]; then
    echo -e "Updating installer.iss to version ${YELLOW}$new_version${RESET}..."
    if [[ "$OSTYPE" == "darwin"* ]]; then
        sed -i '' -E 's/^#define AppVersion .*/#define AppVersion "'"$new_version"'"/' installer.iss
    else
        sed -i -E 's/^#define AppVersion .*/#define AppVersion "'"$new_version"'"/' installer.iss
    fi
fi

# 4. Run cargo check to update Cargo.lock
echo -e "${YELLOW}Running cargo check to update Cargo.lock...${RESET}"
if ! cargo check; then
    echo -e "${RED}[ERROR]${RESET} Cargo check failed. Reverting Cargo.toml..."
    git checkout Cargo.toml
    exit 1
fi

# 5. Git Commit and Tag Confirmation
branch=$(git branch --show-current)
if [[ -z "$branch" ]]; then
    branch="main"
fi

echo -e "\n${YELLOW}Summary of actions to perform:${RESET}"
echo -e "  - Stage and commit changes (Cargo.toml, Cargo.lock, installer.iss)"
echo -e "  - Create git tag v$new_version"
echo -e "  - Push commit and tag to origin ($branch)"
echo ""

read -rp "Are you sure you want to commit, tag, and push? (y/n): " confirm
if [[ "$confirm" != "y" && "$confirm" != "Y" ]]; then
    echo -e "${YELLOW}Operation cancelled. Cargo.toml/Cargo.lock/installer.iss were updated but no Git changes were committed or pushed.${RESET}"
    exit 0
fi

# Commit and tag
echo -e "${YELLOW}Staging changes...${RESET}"
git add Cargo.toml Cargo.lock installer.iss
git commit -m "Bump version to v$new_version"

echo -e "${YELLOW}Creating git tag v$new_version...${RESET}"
git tag -a "v$new_version" -m "Release v$new_version"

# Push to origin
echo -e "${YELLOW}Pushing commits and tag to origin...${RESET}"
if git push origin "$branch" && git push origin "v$new_version"; then
    echo -e "${GREEN}Successfully bumped version to v$new_version and pushed to GitHub!${RESET}"
    echo "GitHub Actions will now compile and publish the release."
else
    echo -e "${RED}[ERROR]${RESET} Failed to push to GitHub. Check your connection or repository permissions."
    echo -e "Note: The commit and tag were created locally. You can push manually using:"
    echo -e "  git push origin $branch"
    echo -e "  git push origin v$new_version"
fi
