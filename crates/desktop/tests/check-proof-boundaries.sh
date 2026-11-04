#!/bin/sh
set -eu
cd "$(dirname "$0")/.."
test_dir=$(mktemp -d /tmp/videodock-proof-check.XXXXXX)
c++ -std=c++17 -fPIC -Iinclude $(pkg-config --cflags Qt6WebEngineQuick Qt6WebEngineWidgets Qt6Network Qt6Widgets) tests/proof_boundaries.cpp src/attestation.cpp src/platform.cpp -o "$test_dir/check" $(pkg-config --libs Qt6WebEngineQuick Qt6WebEngineWidgets Qt6Network Qt6Widgets)
"$test_dir/check"
