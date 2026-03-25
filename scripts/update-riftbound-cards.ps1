# Usage: .\scripts\update-riftbound-cards.ps1 <path-to-riftbound-cards.json>
# Example: .\scripts\update-riftbound-cards.ps1 "C:\Users\dirac\software\orulings\scripts\riftbound_cards_with_errata.json"
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

Write-Host "==> Uploading $(Split-Path $InputFile -Leaf) to server..."
scp $InputFile "${Server}:/opt/riftbound-cards.json"
if ($LASTEXITCODE -ne 0) { Write-Error "scp failed"; exit 1 }

Write-Host "==> Bumping RIFTBOUND_CARDS_VERSION to $Version and restarting service..."
$RemoteScript = @"
sed -i 's/Environment=RIFTBOUND_CARDS_VERSION=.*/Environment=RIFTBOUND_CARDS_VERSION=$Version/' /etc/systemd/system/judge-api.service
systemctl daemon-reload
systemctl restart judge-api
systemctl is-active judge-api
"@
ssh $Server $RemoteScript
if ($LASTEXITCODE -ne 0) { Write-Error "ssh command failed"; exit 1 }

Write-Host "==> Done! Riftbound cards updated to version $Version"
Write-Host "    Verify: curl http://164.92.121.20:3000/riftbound/version"
