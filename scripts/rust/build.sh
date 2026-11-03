#!/bin/sh
set -eu
project=$(CDPATH= cd -- "$(dirname -- "$0")/../.." && pwd)
export CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-2}"
case "$(uname -s)" in
  Darwin)
    export PATH="/opt/homebrew/opt/qtbase/bin:/opt/homebrew/opt/qttools/bin:/opt/homebrew/opt/qtdeclarative/bin:/usr/local/opt/qtbase/bin:/usr/local/opt/qttools/bin:$PATH"
    export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$HOME/Library/Caches/VideoDock/build}"
    ;;
  Linux) export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$HOME/.cache/videodock-rust-target}" ;;
  *) echo 'Use build-windows.ps1 on Windows' >&2; exit 1 ;;
esac
command -v cargo >/dev/null
command -v ffmpeg >/dev/null
command -v ffprobe >/dev/null
cd "$project"
cargo build --locked --release -p videodock-desktop
printf '%s\n' "$CARGO_TARGET_DIR/release/videodock-desktop"
