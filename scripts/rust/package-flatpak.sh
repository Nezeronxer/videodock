#!/bin/sh
set -eu
if [ "$#" -ne 3 ]; then
    echo 'Usage: package-flatpak.sh BASEAPP_BUILD SDK_BINARY OUTPUT_DIRECTORY' >&2
    exit 2
fi
project=$(CDPATH= cd -- "$(dirname -- "$0")/../.." && pwd)
appstreamcli validate --no-net "$project/packaging/linux/io.github.Nezeronxer.VideoDock.metainfo.xml"
base=$(realpath "$1")
binary=$(realpath "$2")
test -x "$binary"
test -f "$base/files/lib/libexec/QtWebEngineProcess"
python3 "$project/LICENSES/generate-rust-licenses.py" --check
mkdir -p "$3"
output=$(realpath "$3")
stage=$(mktemp -d "$output/flatpak-stage.XXXXXX")
repository=$(mktemp -d "$output/flatpak-repo.XXXXXX")
cp -a --reflink=auto "$base/files" "$stage/files"
cp "$project/packaging/linux/flatpak-metadata" "$stage/metadata"
install -m755 "$binary" "$stage/files/bin/videodock"
install -m644 "$project/packaging/linux/qt.conf" "$stage/files/bin/qt.conf"
mkdir -p "$stage/files/share/licenses/videodock" "$stage/files/share/applications" "$stage/files/share/metainfo" "$stage/files/share/icons/hicolor/scalable/apps"
cp -a "$project/LICENSES/." "$stage/files/share/licenses/videodock/"
cp "$project/THIRD-PARTY-NOTICES.md" "$stage/files/share/licenses/videodock/"
cp "$project/crates/desktop/attestation/bgutils/LICENSE" "$stage/files/share/licenses/videodock/BgUtils-MIT.txt"
cp "$project/crates/core/src/sabr/LICENSE.googlevideo" "$stage/files/share/licenses/videodock/googlevideo-MIT.txt"
cp "$project/crates/core/src/sabr/LICENSE.sabr-rs" "$stage/files/share/licenses/videodock/sabr-rs-MIT.txt"
cp "$project/assets/videodock.svg" "$stage/files/share/icons/hicolor/scalable/apps/io.github.Nezeronxer.VideoDock.svg"
cp "$project/packaging/linux/io.github.Nezeronxer.VideoDock.metainfo.xml" "$stage/files/share/metainfo/"
python3 - "$stage/files/share/applications/io.github.Nezeronxer.VideoDock.desktop" <<'PY'
from pathlib import Path
import sys
Path(sys.argv[1]).write_text('''[Desktop Entry]
Type=Application
Name=VideoDock
Comment=Download videos and audio
Comment[ru]=Скачивание видео и аудио
Exec=videodock
Icon=io.github.Nezeronxer.VideoDock
Terminal=false
Categories=AudioVideo;Network;
''')
PY
version=$(python3 -c 'import sys,tomllib; print(tomllib.load(open(sys.argv[1],"rb"))["workspace"]["package"]["version"])' "$project/Cargo.toml")
flatpak build-finish "$stage"
flatpak build-export "$repository" "$stage" stable
bundle="$output/VideoDock-$version-linux-x86_64.flatpak"
flatpak build-bundle --runtime-repo=https://flathub.org/repo/flathub.flatpakrepo "$repository" "$bundle" io.github.Nezeronxer.VideoDock stable
sha256sum "$bundle"
