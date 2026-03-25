# Builds the judge-api for Linux using Docker and deploys it to the server.
$Server = "root@164.92.121.20"

Write-Host "==> Building judge-api via Docker..."
docker run --rm -v "${PWD}:/app" -w /app rust:alpine sh -c "apk add musl-dev && cargo build --release -p judge-api --target x86_64-unknown-linux-musl"
if ($LASTEXITCODE -ne 0) { Write-Error "Build failed"; exit 1 }

Write-Host "==> Uploading binary..."
scp target/x86_64-unknown-linux-musl/release/judge-api "${Server}:/opt/judge-api"
if ($LASTEXITCODE -ne 0) { Write-Error "Upload failed"; exit 1 }

Write-Host "==> Restarting service..."
ssh $Server "systemctl restart judge-api && systemctl is-active judge-api"

Write-Host "==> Done! Verify: curl http://164.92.121.20:3000/riftbound/version"
