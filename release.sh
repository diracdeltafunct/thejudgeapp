#!/usr/bin/env bash
set -e

TAURI_CONF="src-tauri/tauri.conf.json"
TAURI_PROPS="src-tauri/gen/android/app/tauri.properties"

# --- Read current values ---
current_version=$(grep '"version"' "$TAURI_CONF" | head -1 | sed 's/.*"version": "\(.*\)".*/\1/')
current_version_code=$(grep 'tauri.android.versionCode' "$TAURI_PROPS" | sed 's/tauri.android.versionCode=//')

new_version_code=$((current_version_code + 1))

# --- Prompt for new version name ---
echo ""
echo "Current version : $current_version  (versionCode $current_version_code)"
echo "New versionCode will be: $new_version_code"
echo ""

# Suggest next patch version
IFS='.' read -r major minor patch <<< "$current_version"
suggested="${major}.${minor}.$((patch + 1))"

read -rp "Enter new version name [$suggested]: " new_version
new_version="${new_version:-$suggested}"

echo ""
echo "Updating to: $new_version (versionCode $new_version_code)"
echo ""

# --- Update files ---
# tauri.conf.json
sed -i "s/\"version\": \"$current_version\"/\"version\": \"$new_version\"/" "$TAURI_CONF"

# tauri.properties
sed -i "s/tauri.android.versionName=.*/tauri.android.versionName=$new_version/" "$TAURI_PROPS"
sed -i "s/tauri.android.versionCode=.*/tauri.android.versionCode=$new_version_code/" "$TAURI_PROPS"

echo "Files updated."

# --- Require clean commit ---
echo ""
git status --short
echo ""
echo "All changes must be committed before tagging."
read -rp "Enter a commit message (or leave blank to abort): " commit_msg

if [ -z "$commit_msg" ]; then
  echo "Aborted — no commit message provided."
  exit 1
fi

git add -A
git commit -m "$commit_msg"

# --- Tag ---
tag="v$new_version"
echo ""
echo "Tagging commit as $tag"
git tag "$tag"
echo "Tagged."

# --- Release notes ---
prev_tag=$(git tag --sort=-version:refname | grep -v "^$tag$" | head -1)
notes_file="${new_version}releasenotes.txt"

echo ""
if [ -n "$prev_tag" ]; then
  echo "Collecting commits since $prev_tag..."
  {
    echo "Release $new_version"
    echo "======================"
    git log "${prev_tag}..HEAD" --pretty=format:"- %s" --no-merges
    echo ""
  } > "$notes_file"
else
  echo "No previous tag found — collecting all commits..."
  {
    echo "Release $new_version"
    echo "======================"
    git log --pretty=format:"- %s" --no-merges
    echo ""
  } > "$notes_file"
fi

echo "Release notes written to $notes_file"

# --- Build ---
echo ""
echo "Starting release build..."
echo ""
npm run tauri android build
