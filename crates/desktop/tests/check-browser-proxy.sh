#!/bin/sh
set -eu
cd "$(dirname "$0")/.."
test_dir=$(mktemp -d /tmp/videodock-browser-proxy.XXXXXX)
/usr/lib/qt6/moc tests/browser_proxy.cpp -o "$test_dir/browser_proxy.moc"
c++ -std=c++17 -fPIC -Iinclude -I"$test_dir" $(pkg-config --cflags Qt6WebEngineQuick Qt6WebEngineCore Qt6Network Qt6Widgets) tests/browser_proxy.cpp src/platform.cpp -o "$test_dir/check" $(pkg-config --libs Qt6WebEngineQuick Qt6WebEngineCore Qt6Network Qt6Widgets)
for mode in 0 1 2; do
    xvfb-run -a env QT_QPA_PLATFORM=xcb LIBGL_ALWAYS_SOFTWARE=1 QTWEBENGINE_CHROMIUM_FLAGS=--disable-gpu QT_FORCE_STDERR_LOGGING=1 "$test_dir/check" "$mode"
done
