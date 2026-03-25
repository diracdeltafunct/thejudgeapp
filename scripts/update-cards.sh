#!/usr/bin/env bash
# Usage: ./scripts/update-cards.sh <path-to-all-cards.json>
# Example: ./scripts/update-cards.sh "C:/Users/dirac/Downloads/all-cards-20260324092145.json"
set -e

SERVER="root@164.92.121.20"
INPUT="$1"
VERSION=$(date +%Y%m%d%H%M%S)

if [ -z "$INPUT" ]; then
  echo "Error: provide path to all-cards JSON as first argument"
  echo "Usage: $0 <path-to-all-cards.json>"
  exit 1
fi

if [ ! -f "$INPUT" ]; then
  echo "Error: file not found: $INPUT"
  exit 1
fi

echo "==> Building compile-cards..."
cargo build --release -p judge-api --bin compile-cards

echo "==> Compiling cards (version $VERSION)..."
cargo run --release -p judge-api --bin compile-cards -- "$INPUT" judge-cards.json

echo "==> Uploading judge-cards.json to server..."
scp judge-cards.json "$SERVER":/opt/judge-cards.json

echo "==> Bumping CARDS_VERSION to $VERSION and restarting service..."
ssh "$SERVER" bash << EOF
  sed -i 's/Environment=CARDS_VERSION=.*/Environment=CARDS_VERSION=$VERSION/' /etc/systemd/system/judge-api.service
  systemctl daemon-reload
  systemctl restart judge-api
  echo "Service status:"
  systemctl is-active judge-api
EOF

echo "==> Done! Cards updated to version $VERSION"
echo "    Verify: curl http://$SERVER:3000/version"
