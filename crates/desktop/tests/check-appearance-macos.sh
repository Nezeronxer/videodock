#!/bin/sh
# Native Cocoa fixture; all files and Engine state remain in the caller's QA directory.
set -eu
project=$(CDPATH='' cd -- "$(dirname -- "$0")/../../.." && pwd)
output=${1:?Expected an absolute QA output directory}
case "$output" in /*) ;; *) echo 'Expected an absolute QA output directory' >&2; exit 2 ;; esac
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-"$HOME/Library/Caches/VideoDock/build"}"
build="$CARGO_TARGET_DIR/appearance"
mkdir -p "$output"
cd "$project"
cargo build -p videodock-core --example ui_engine --locked
cmake -S crates/desktop/tests -B "$build" -G Ninja "-DCMAKE_PREFIX_PATH=$(qmake -query QT_INSTALL_PREFIX)" -DCMAKE_BUILD_TYPE=Release
cmake --build "$build" --parallel 2
ffmpeg -hide_banner -nostdin -loglevel error -y -f lavfi -i color=c=blue:s=160x90:r=10 -f lavfi -i sine=frequency=440:sample_rate=44100 -t 1 -c:v libx264 -pix_fmt yuv420p -c:a aac "$output/sample.mp4"
failed=0
for language in en ru; do
    mkdir -p "$output/$language"
    if QT_QPA_PLATFORM=cocoa QT_FORCE_STDERR_LOGGING=1 "$build/videodock-appearance" "$project/crates/desktop/qml/Main.qml" "$output/$language" "$language" "$CARGO_TARGET_DIR/debug/examples/ui_engine" "$output/sample.mp4" > "$output/$language.log" 2>&1; then
        printf '%s: PASS\n' "$language"
    else
        failed=1
        printf '%s: FAIL; see %s\n' "$language" "$output/$language.log" >&2
    fi
done
exit "$failed"
