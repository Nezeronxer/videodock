#!/bin/sh
set -eu
cd "$(dirname "$0")/.."
test_dir=$(mktemp -d /tmp/videodock-appearance.XXXXXX)
output_dir=${1:-"$test_dir/screenshots"}
mkdir -p "$output_dir"
CARGO_TARGET_DIR=${CARGO_TARGET_DIR:-"$HOME/.cache/videodock-rust-target"}
export CARGO_TARGET_DIR
CARGO_BUILD_JOBS=2 cargo build -p videodock-core --example ui_engine --offline
ffmpeg -hide_banner -loglevel error -f lavfi -i color=c=blue:s=160x90:r=10 -f lavfi -i sine=frequency=440:sample_rate=44100 -t 1 -c:v libx264 -pix_fmt yuv420p -c:a aac "$test_dir/sample.mp4"
/usr/lib/qt6/moc tests/appearance.cpp -o "$test_dir/appearance.moc"
c++ -std=c++17 -fPIC -I"$test_dir" $(pkg-config --cflags Qt6Quick Qt6QuickControls2 Qt6Test Qt6WebEngineQuick Qt6Widgets) tests/appearance.cpp -o "$test_dir/check" $(pkg-config --libs Qt6Quick Qt6QuickControls2 Qt6Test Qt6WebEngineQuick Qt6Widgets)
xvfb-run -a -s '-screen 0 1600x1000x24' env QT_QPA_PLATFORM=xcb LIBGL_ALWAYS_SOFTWARE=1 QTWEBENGINE_CHROMIUM_FLAGS=--disable-gpu QT_FORCE_STDERR_LOGGING=1 "$test_dir/check" "$PWD/qml/Main.qml" "$output_dir" "${2:-en}" "$CARGO_TARGET_DIR/debug/examples/ui_engine" "$test_dir/sample.mp4" "${VIDEODOCK_REFERENCE_PACK:-}"
