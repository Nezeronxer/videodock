#!/bin/sh
set -eu
project=$(CDPATH= cd -- "$(dirname -- "$0")/../.." && pwd)
case "$(uname -s)" in
  Linux)
    build="${CARGO_TARGET_DIR:-$HOME/.cache/videodock-rust-target}"
    binary="$build/release/videodock-desktop"
    "$project/scripts/rust/build.sh"
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
    app="${VIDEODOCK_MAC_APP:-$HOME/Applications/VideoDock.app}"
    case "$app" in /*.app) ;; *) echo 'Expected an absolute .app destination' >&2; exit 1 ;; esac
    "$project/scripts/rust/build.sh"
    mkdir -p "$app/Contents/MacOS" "$app/Contents/Resources"
    cp "$binary" "$app/Contents/MacOS/VideoDock"
    for tool in ffmpeg ffprobe; do
        install -m755 "$(command -v "$tool")" "$app/Contents/MacOS/$tool"
    done
    cat > "$app/Contents/Info.plist" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict><key>CFBundleName</key><string>VideoDock</string><key>CFBundleIdentifier</key><string>com.nezeronxer.videodock</string><key>CFBundleExecutable</key><string>VideoDock</string><key>CFBundlePackageType</key><string>APPL</string><key>CFBundleShortVersionString</key><string>0.2.0</string><key>CFBundleVersion</key><string>0.2.0</string><key>NSHighResolutionCapable</key><true/></dict></plist>
EOF
    command -v macdeployqt >/dev/null
    set -- "$app" -qmldir="$project/crates/desktop/qml" -executable="$app/Contents/MacOS/ffmpeg" -executable="$app/Contents/MacOS/ffprobe" -always-overwrite
    for qtlib in /opt/homebrew/opt/qt[0-9a-z]*/lib /usr/local/opt/qt[0-9a-z]*/lib; do
        if [ -d "$qtlib" ]; then set -- "$@" "-libpath=$qtlib"; fi
    done
    macdeployqt "$@"
    plutil -lint "$app/Contents/Info.plist"
    for tool in VideoDock ffmpeg ffprobe; do
        dependencies=$(otool -L "$app/Contents/MacOS/$tool")
        if printf '%s\n' "$dependencies" | awk 'NR>1 && $1 ~ /^\/(opt\/homebrew|usr\/local|Users)\// {bad=1} END {exit !bad}'; then
            echo "Unbundled library dependency in $tool" >&2
            exit 1
        fi
    done
    for library in "$app/Contents/Frameworks/"*.dylib; do
        [ -f "$library" ] || continue
        # Homebrew dylibs retain ../lib rpaths after Qt flattens them into Frameworks.
        if ! otool -l "$library" | awk '$1 == "path" && $2 == "@loader_path" {found=1} END {exit !found}'; then
            install_name_tool -add_rpath '@loader_path' "$library"
        fi
        codesign --force --sign - "$library"
    done
    codesign --force --sign - "$app"
    codesign --verify --deep --strict "$app"
    "$app/Contents/MacOS/ffmpeg" -version >/dev/null
    "$app/Contents/MacOS/ffprobe" -version >/dev/null
    ;;
  *) echo 'Use build-windows.ps1 on Windows' >&2; exit 1 ;;
esac
printf '%s\n' 'VideoDock installed. FFmpeg/FFprobe must be available in PATH.'
