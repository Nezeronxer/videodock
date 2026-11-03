#!/bin/sh
set -eu
cd "$(dirname "$0")/.."
test_dir=$(mktemp -d /tmp/videodock-cookie-check.XXXXXX)
/usr/lib/qt6/moc tests/cookie_bridge.cpp -o "$test_dir/cookie_bridge.moc"
c++ -std=c++17 -fPIC -Iinclude -I"$test_dir" $(pkg-config --cflags Qt6WebEngineQuick Qt6WebEngineCore Qt6Network Qt6Widgets) tests/cookie_bridge.cpp src/platform.cpp -o "$test_dir/check" $(pkg-config --libs Qt6WebEngineQuick Qt6WebEngineCore Qt6Network Qt6Widgets)
xvfb-run -a env QT_QPA_PLATFORM=xcb LIBGL_ALWAYS_SOFTWARE=1 QTWEBENGINE_CHROMIUM_FLAGS=--disable-gpu QT_FORCE_STDERR_LOGGING=1 "$test_dir/check"
