#!/usr/bin/env bash
# Usage: ./scripts/update-riftbound-cards.sh <path-to-riftbound-cards.json>
# Example: ./scripts/update-riftbound-cards.sh "C:/Users/dirac/software/orulings/scripts/riftbound_cards_with_errata.json"
set -e

SERVER="root@164.92.121.20"
INPUT="$1"
VERSION=$(date +%Y%m%d%H%M%S)

if [ -z "$INPUT" ]; then
  echo "Error: provide path to riftbound cards JSON as first argument"
  echo "Usage: $0 <path-to-riftbound-cards.json>"
  exit 1
fi

if [ ! -f "$INPUT" ]; then
  echo "Error: file not found: $INPUT"
  exit 1
fi

echo "==> Uploading $(basename "$INPUT") to server..."
scp "$INPUT" "$SERVER":/opt/riftbound-cards.json

echo "==> Bumping RIFTBOUND_CARDS_VERSION to $VERSION and restarting service..."
ssh "$SERVER" bash << EOF
  sed -i 's/Environment=RIFTBOUND_CARDS_VERSION=.*/Environment=RIFTBOUND_CARDS_VERSION=$VERSION/' /etc/systemd/system/judge-api.service
  systemctl daemon-reload
  systemctl restart judge-api
  echo "Service status:"
  systemctl is-active judge-api
EOF

echo "==> Done! Riftbound cards updated to version $VERSION"
echo "    Verify: curl http://164.92.121.20:3000/riftbound/version"
