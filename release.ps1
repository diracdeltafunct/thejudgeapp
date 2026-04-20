$ErrorActionPreference = "Stop"

$TAURI_CONF = "src-tauri/tauri.conf.json"
$TAURI_PROPS = "src-tauri/gen/android/app/tauri.properties"

# --- Read current values ---
$confContent = Get-Content $TAURI_CONF -Raw
if ($confContent -match '"version":\s*"([^"]+)"') {
    $currentVersion = $Matches[1]
} else {
    Write-Error "Could not read version from $TAURI_CONF"
    exit 1
}

$propsContent = Get-Content $TAURI_PROPS -Raw
if ($propsContent -match 'tauri\.android\.versionCode=(\d+)') {
    $currentVersionCode = [int]$Matches[1]
} else {
    Write-Error "Could not read versionCode from $TAURI_PROPS"
    exit 1
}

$newVersionCode = $currentVersionCode + 1

# --- Prompt for new version name ---
Write-Host ""
Write-Host "Current version : $currentVersion  (versionCode $currentVersionCode)"
Write-Host "New versionCode will be: $newVersionCode"
Write-Host ""

$parts = $currentVersion -split '\.'
$suggested = "$($parts[0]).$($parts[1]).$([int]$parts[2] + 1)"

$newVersion = Read-Host "Enter new version name [$suggested]"
if ([string]::IsNullOrWhiteSpace($newVersion)) { $newVersion = $suggested }

Write-Host ""
Write-Host "Updating to: $newVersion (versionCode $newVersionCode)"
Write-Host ""

# --- Update files ---
$confContent = $confContent -replace '"version":\s*"[^"]+"', "`"version`": `"$newVersion`""
Set-Content $TAURI_CONF $confContent -NoNewline

$propsContent = $propsContent -replace 'tauri\.android\.versionName=.*', "tauri.android.versionName=$newVersion"
$propsContent = $propsContent -replace 'tauri\.android\.versionCode=.*', "tauri.android.versionCode=$newVersionCode"
Set-Content $TAURI_PROPS $propsContent -NoNewline

Write-Host "Files updated."

# --- Require clean commit ---
Write-Host ""
git status --short
Write-Host ""
Write-Host "All changes must be committed before tagging."
$commitMsg = Read-Host "Enter a commit message (or leave blank to abort)"

if ([string]::IsNullOrWhiteSpace($commitMsg)) {
    Write-Host "Aborted — no commit message provided."
    exit 1
}

git add -A
git commit -m $commitMsg

# --- Tag ---
$tag = "v$newVersion"
Write-Host ""
Write-Host "Tagging commit as $tag"
git tag $tag
Write-Host "Tagged."

# --- Release notes ---
$prevTag = git tag --sort=-version:refname | Where-Object { $_ -ne $tag } | Select-Object -First 1
$notesFile = "resources/" + $newVersion + "releasenotes.txt"

Write-Host ""
if ($prevTag) {
    Write-Host "Collecting commits since $prevTag..."
    $lines = @("Release $newVersion", "======================")
    $lines += git log "$($prevTag)..HEAD" --pretty=format:"- %s" --no-merges
    $lines += ""
} else {
    Write-Host "No previous tag found — collecting all commits..."
    $lines = @("Release $newVersion", "======================")
    $lines += git log --pretty=format:"- %s" --no-merges
    $lines += ""
}


Write-Host "Release notes written to $notesFile"

# --- Build ---
Write-Host ""
Write-Host "Starting release build..."
Write-Host ""
npm run tauri android build
