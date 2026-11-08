#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")" && pwd)"
PORTABLE="$ROOT/dist-linux"
BIN="$PORTABLE/discord-quest-completer"

if [[ ! -x "$BIN" ]]; then
  echo "Binary not found. Building production app (this may take a few minutes)…"
  # shellcheck disable=SC1090
  source "$HOME/.cargo/env"
  cd "$ROOT"
  pnpm sync:runner
  # Must use `tauri build`, not plain `cargo build` — otherwise UI points at localhost:1420
  export CARGO_TARGET_DIR="$ROOT/src-tauri/target"
  pnpm tauri build
  mkdir -p "$PORTABLE"
  cp "$ROOT/src-tauri/target/release/discord-quest-completer" "$PORTABLE/"
  cp -a "$ROOT/src-tauri/target/release/data" "$PORTABLE/"
fi

cd "$PORTABLE"
export WEBKIT_DISABLE_DMABUF_RENDERER="${WEBKIT_DISABLE_DMABUF_RENDERER:-1}"
export WEBKIT_DISABLE_COMPOSITING_MODE="${WEBKIT_DISABLE_COMPOSITING_MODE:-1}"
export GDK_BACKEND="${GDK_BACKEND:-x11}"
exec ./discord-quest-completer "$@"
