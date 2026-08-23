#!/usr/bin/env bash
set -Eeuo pipefail

TAURI_CONF="src-tauri/tauri.conf.json"
ANDROID_CONF="src-tauri/tauri.android.conf.json"
NOTES_DIR="resources"

fail() {
  echo "Error: $*" >&2
  exit 1
}

command -v git >/dev/null || fail "git is not installed"
command -v node >/dev/null || fail "Node.js is not installed"
command -v npm >/dev/null || fail "npm is not installed"
command -v cargo >/dev/null || fail "Rust/Cargo is not installed"

[[ -f "$TAURI_CONF" ]] || fail "$TAURI_CONF does not exist"
[[ -f "$ANDROID_CONF" ]] || fail "$ANDROID_CONF does not exist"
[[ -n "${JAVA_HOME:-}" && -d "$JAVA_HOME" ]] || fail "JAVA_HOME is not set to an installed JDK"
[[ -n "${ANDROID_HOME:-}" && -d "$ANDROID_HOME" ]] || fail "ANDROID_HOME is not set to the Android SDK"
[[ -n "${NDK_HOME:-}" && -d "$NDK_HOME" ]] || fail "NDK_HOME is not set to an installed Android NDK"
[[ -d "$ANDROID_HOME/platforms/android-36" ]] || fail "Android SDK Platform 36 is not installed"

if [[ -n "${KEYSTORE_PATH:-}" ]]; then
  [[ -f "$KEYSTORE_PATH" ]] || fail "KEYSTORE_PATH does not exist: $KEYSTORE_PATH"
elif [[ ! -f src-tauri/gen/android/key.properties && ! -f src-tauri/gen/android/app/thejudgeapp.jks ]]; then
  fail "Android release signing is not configured. Set KEYSTORE_PATH and the signing password variables, or create src-tauri/gen/android/key.properties."
fi

current_version=$(node -e '
  const config = require("./src-tauri/tauri.conf.json");
  if (!config.version) process.exit(1);
  process.stdout.write(config.version);
') || fail "Could not read the current version"

if [[ ! "$current_version" =~ ^([0-9]+)\.([0-9]+)\.([0-9]+)$ ]]; then
  fail "Current version is not semantic versioning: $current_version"
fi

major=${BASH_REMATCH[1]}
minor=${BASH_REMATCH[2]}
patch_version=${BASH_REMATCH[3]}

current_version_code=$(node -e '
  const base = require("./src-tauri/tauri.conf.json");
  const android = require("./src-tauri/tauri.android.conf.json");
  if (android.bundle?.android?.versionCode) {
    process.stdout.write(String(android.bundle.android.versionCode));
  } else {
    const [major, minor, patch] = base.version.split(".").map(Number);
    process.stdout.write(String(major * 1000000 + minor * 1000 + patch));
  }
') || fail "Could not determine the Android version code"

[[ "$current_version_code" =~ ^[0-9]+$ ]] || fail "Invalid Android version code: $current_version_code"
new_version_code=$((current_version_code + 1))
suggested="${major}.${minor}.$((patch_version + 1))"

echo
echo "Current version : $current_version (versionCode $current_version_code)"
echo "New versionCode : $new_version_code"
echo
read -rp "Enter new version name [$suggested]: " new_version
new_version="${new_version:-$suggested}"

[[ "$new_version" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]] || fail "Version must use major.minor.patch"
tag="v$new_version"
git rev-parse -q --verify "refs/tags/$tag" >/dev/null && fail "Tag $tag already exists"

echo
echo "Updating to $new_version (versionCode $new_version_code)"

node - "$TAURI_CONF" "$ANDROID_CONF" "$new_version" "$new_version_code" <<'NODE'
const fs = require("fs");
const [tauriPath, androidPath, version, versionCodeRaw] = process.argv.slice(2);

const tauri = JSON.parse(fs.readFileSync(tauriPath, "utf8"));
tauri.version = version;
fs.writeFileSync(tauriPath, `${JSON.stringify(tauri, null, 2)}\n`);

const android = JSON.parse(fs.readFileSync(androidPath, "utf8"));
android.bundle ??= {};
android.bundle.android ??= {};
android.bundle.android.versionCode = Number(versionCodeRaw);
fs.writeFileSync(androidPath, `${JSON.stringify(android, null, 2)}\n`);
NODE

echo
git status --short
echo
echo "All displayed changes will be included in the release commit."
read -rp "Enter a commit message (or leave blank to abort): " commit_msg
[[ -n "$commit_msg" ]] || fail "No commit message provided; version files remain modified"

mkdir -p "$NOTES_DIR"
prev_tag=$(git tag --sort=-version:refname | grep -v "^${tag}$" | head -1 || true)
notes_file="$NOTES_DIR/${new_version}releasenotes.txt"

{
  echo "Release $new_version"
  echo "======================"
  echo "- $commit_msg"
  if [[ -n "$prev_tag" ]]; then
    git log "${prev_tag}..HEAD" --pretty=format:"- %s" --no-merges
  else
    git log --pretty=format:"- %s" --no-merges
  fi
  echo
} > "$notes_file"

echo "Release notes written to $notes_file"
git add -A
git commit -m "$commit_msg"

echo
echo "Starting signed Android release build..."
npm run tauri android build

echo
echo "Tagging successful build as $tag"
git tag "$tag"

echo
echo "Release $tag built and tagged successfully."
echo "Artifacts are under src-tauri/gen/android/app/build/outputs/."
