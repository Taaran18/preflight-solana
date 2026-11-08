#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

[ -f "$ROOT/backend/.env" ] || cp "$ROOT/backend/.env.example" "$ROOT/backend/.env"
[ -f "$ROOT/frontend/.env.local" ] || cp "$ROOT/frontend/.env.example" "$ROOT/frontend/.env.local"
[ -d "$ROOT/frontend/node_modules" ] || (cd "$ROOT/frontend" && npm install)

(cd "$ROOT/backend" && cargo run --release) &
API_PID=$!

cleanup() {
  kill "$API_PID" 2>/dev/null || true
}
trap cleanup EXIT INT TERM

cd "$ROOT/frontend"
npm run dev
