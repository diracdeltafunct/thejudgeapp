# Usage: .\scripts\update-cards.ps1 <path-to-all-cards.json>
# Example: .\scripts\update-cards.ps1 "C:\Users\dirac\Downloads\all-cards-20260410092122.json"
param(
    [Parameter(Mandatory=$true)]
    [string]$InputFile
)

$Server = "root@164.92.121.20"
$Version = (Get-Date -Format "yyyyMMddHHmmss")

if (-not (Test-Path $InputFile)) {
    Write-Error "File not found: $InputFile"
    exit 1
}

Write-Host "==> Building compile-cards..."
cargo build --release -p judge-api --bin compile-cards
if ($LASTEXITCODE -ne 0) { Write-Error "Build failed"; exit 1 }

Write-Host "==> Compiling cards (version $Version)..."
cargo run --release -p judge-api --bin compile-cards -- $InputFile judge-cards.json
if ($LASTEXITCODE -ne 0) { Write-Error "Compile failed"; exit 1 }

Write-Host "==> Uploading judge-cards.json to server..."
scp judge-cards.json "${Server}:/opt/judge-cards.json"
if ($LASTEXITCODE -ne 0) { Write-Error "Upload failed"; exit 1 }

Write-Host "==> Bumping CARDS_VERSION to $Version and restarting service..."
$RemoteScript = @"
sed -i 's/Environment=CARDS_VERSION=.*/Environment=CARDS_VERSION=$Version/' /etc/systemd/system/judge-api.service
systemctl daemon-reload
systemctl restart judge-api
systemctl is-active judge-api
"@
ssh $Server $RemoteScript
if ($LASTEXITCODE -ne 0) { Write-Error "ssh command failed"; exit 1 }

Write-Host "==> Done! Cards updated to version $Version"
Write-Host "    Verify: curl http://164.92.121.20:3000/version"
