#!/bin/sh
set -eu
cd "$(dirname "$0")/.."
test_dir=$(mktemp -d /tmp/videodock-address-focus.XXXXXX)
trap 'rm -rf -- "$test_dir"' EXIT
c++ -std=c++17 -fPIC $(pkg-config --cflags Qt6Quick Qt6QuickControls2 Qt6Test Qt6Widgets) tests/browser-address-focus.cpp -o "$test_dir/check" $(pkg-config --libs Qt6Quick Qt6QuickControls2 Qt6Test Qt6Widgets)
"$test_dir/check" "$PWD/qml/BrowserPane.qml"
