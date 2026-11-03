#!/bin/sh
set -eu
project=$(CDPATH= cd -- "$(dirname -- "$0")/../.." && pwd)
case "$(uname -s)" in
  Linux)
    build="${CARGO_TARGET_DIR:-$HOME/.cache/videodock-rust-target}"
    binary="$build/release/videodock-desktop"
    [ -x "$binary" ] || "$project/scripts/rust/build.sh"
    install -Dm755 "$binary" "$HOME/.local/lib/videodock/videodock"
    install -Dm644 "$project/assets/videodock.svg" "$HOME/.local/share/icons/hicolor/scalable/apps/videodock.svg"
    mkdir -p "$HOME/.local/bin" "$HOME/.local/share/applications"
    ln -sfn "$HOME/.local/lib/videodock/videodock" "$HOME/.local/bin/videodock"
    executable=$(printf '%s' "$HOME/.local/lib/videodock/videodock" | sed 's/\\/\\\\/g;s/"/\\"/g;s/%/%%/g')
    cat > "$HOME/.local/share/applications/videodock.desktop" <<EOF
[Desktop Entry]
Type=Application
Name=VideoDock
Comment=Video downloader written in Rust
Exec="$executable"
Icon=videodock
Terminal=false
Categories=AudioVideo;Network;
EOF
    ;;
  Darwin)
    export PATH="/opt/homebrew/opt/qtbase/bin:/opt/homebrew/opt/qttools/bin:/usr/local/opt/qtbase/bin:/usr/local/opt/qttools/bin:$PATH"
    build="${CARGO_TARGET_DIR:-$HOME/Library/Caches/VideoDock/build}"
    binary="$build/release/videodock-desktop"
    [ -x "$binary" ] || "$project/scripts/rust/build.sh"
    app="$HOME/Applications/VideoDock.app"
    mkdir -p "$app/Contents/MacOS" "$app/Contents/Resources"
    cp "$binary" "$app/Contents/MacOS/VideoDock"
    cat > "$app/Contents/Info.plist" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict><key>CFBundleName</key><string>VideoDock</string><key>CFBundleIdentifier</key><string>com.nezeronxer.videodock</string><key>CFBundleExecutable</key><string>VideoDock</string><key>CFBundlePackageType</key><string>APPL</string><key>CFBundleShortVersionString</key><string>0.2.0</string><key>CFBundleVersion</key><string>0.2.0</string><key>NSHighResolutionCapable</key><true/></dict></plist>
EOF
    command -v macdeployqt >/dev/null
    macdeployqt "$app" -qmldir="$project/crates/desktop/qml" -always-overwrite
    plutil -lint "$app/Contents/Info.plist"
    ;;
  *) echo 'Use build-windows.ps1 on Windows' >&2; exit 1 ;;
esac
printf '%s\n' 'VideoDock installed. FFmpeg/FFprobe must be available in PATH.'
