#!/bin/sh
set -eu
project=$(CDPATH= cd -- "$(dirname -- "$0")/../.." && pwd)
stage_notices() {
    licenses=$1
    mkdir -p "$licenses"
    cp -R "$project/LICENSES/." "$licenses/"
    cp "$project/THIRD-PARTY-NOTICES.md" "$(dirname "$licenses")/THIRD-PARTY-NOTICES.md"
    cp "$project/crates/desktop/attestation/bgutils/LICENSE" "$licenses/BgUtils-MIT.txt"
    cp "$project/crates/core/src/sabr/LICENSE.googlevideo" "$licenses/googlevideo-MIT.txt"
    cp "$project/crates/core/src/sabr/LICENSE.sabr-rs" "$licenses/sabr-rs-MIT.txt"
    ffmpeg -L > "$licenses/ffmpeg-license.txt" 2>&1
    ffmpeg -buildconf > "$licenses/ffmpeg-buildconf.txt" 2>&1
    if command -v qmake6 >/dev/null; then
        qmake6 -query QT_VERSION > "$licenses/qt-version.txt"
    elif command -v qmake >/dev/null; then
        qmake -query QT_VERSION > "$licenses/qt-version.txt"
    fi
}
install_stage=
install_published=0
cleanup_install() {
    install_result=$?
    if [ -n "$install_stage" ]; then
        if [ "$install_result" -ne 0 ]; then
            if [ "$install_published" -eq 1 ] && { [ -e "$install_destination" ] || [ -L "$install_destination" ]; }; then
                mv "$install_destination" "$install_stage/failed" || return 1
            fi
            if [ -e "$install_stage/previous" ] || [ -L "$install_stage/previous" ]; then
                mv "$install_stage/previous" "$install_destination" || return 1
            fi
        fi
        rm -rf -- "$install_stage"
    fi
    return "$install_result"
}
trap cleanup_install EXIT
trap 'exit 1' HUP INT TERM
stage_install() {
    install_destination=$1
    mkdir -p "$(dirname "$install_destination")"
    install_stage=$(mktemp -d "$(dirname "$install_destination")/.videodock-install.XXXXXX")
    install_candidate="$install_stage/$(basename "$install_destination")"
}
publish_install() {
    if [ -e "$install_destination" ] || [ -L "$install_destination" ]; then
        mv "$install_destination" "$install_stage/previous"
    fi
    install_published=1
    mv "$install_candidate" "$install_destination"
}
case "$(uname -s)" in
  Linux)
    build="${CARGO_TARGET_DIR:-$HOME/.cache/videodock-rust-target}"
    binary="$build/release/videodock-desktop"
    "$project/scripts/rust/build.sh"
    stage_install "$HOME/.local/lib/videodock"
    install -Dm755 "$binary" "$install_candidate/videodock"
    stage_notices "$install_candidate/licenses"
    publish_install
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
    stage_install "$app"
    app=$install_candidate
    mkdir -p "$app/Contents/MacOS" "$app/Contents/Resources"
    cp "$binary" "$app/Contents/MacOS/VideoDock"
    stage_notices "$app/Contents/Resources/licenses"
    if command -v brew >/dev/null; then
        formulae=""
        for formula in qtbase qtdeclarative qtwebengine qttools qt5compat qtsvg ffmpeg-full; do
            prefix=$(brew --prefix "$formula")
            if [ -d "$prefix" ]; then formulae="$formulae $formula"; fi
        done
        if [ -n "$formulae" ]; then
            dependencies=$(brew deps --union --installed $formulae)
            for formula in $(printf '%s\n' $formulae $dependencies | sort -u); do
                prefix=$(brew --prefix "$formula")
                destination="$licenses/Homebrew/$formula"
                mkdir -p "$destination"
                for notice in "$prefix"/LICENSE* "$prefix"/LICENCE* "$prefix"/COPYING* "$prefix"/NOTICE* "$prefix"/AUTHORS* "$prefix"/.brew; do
                    if [ -e "$notice" ]; then cp -R "$notice" "$destination/"; fi
                done
            done
        fi
    fi
    for tool in ffmpeg ffprobe; do
        install -m755 "$(command -v "$tool")" "$app/Contents/MacOS/$tool"
    done
    cp "$project/assets/videodock.icns" "$app/Contents/Resources/VideoDock.icns"
    version=$(sed -n 's/^version = "\([0-9][0-9.]*\)"$/\1/p' "$project/Cargo.toml")
    [ -n "$version" ] || { echo "Workspace release version absent" >&2; exit 1; }
    cat > "$app/Contents/Info.plist" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict><key>CFBundleName</key><string>VideoDock</string><key>CFBundleIdentifier</key><string>com.nezeronxer.videodock</string><key>CFBundleExecutable</key><string>VideoDock</string><key>CFBundleIconFile</key><string>VideoDock.icns</string><key>CFBundlePackageType</key><string>APPL</string><key>CFBundleShortVersionString</key><string>$version</string><key>CFBundleVersion</key><string>$version</string><key>NSHighResolutionCapable</key><true/></dict></plist>
EOF
    command -v macdeployqt >/dev/null
    identity='VideoDock Self-Signed'
    set -- "$app" -qmldir="$project/crates/desktop/qml" -executable="$app/Contents/MacOS/ffmpeg" -executable="$app/Contents/MacOS/ffprobe" -always-overwrite
    for qtlib in /opt/homebrew/opt/qt[0-9a-z]*/lib /usr/local/opt/qt[0-9a-z]*/lib; do
        if [ -d "$qtlib" ]; then
            set -- "$@" "-libpath=$qtlib"
            # macdeployqt resolves QML @rpath imports from executable rpaths, not -libpath.
            if ! otool -l "$app/Contents/MacOS/VideoDock" | awk -v expected="$qtlib" '$1 == "path" && $2 == expected {found=1} END {exit !found}'; then
                install_name_tool -add_rpath "$qtlib" "$app/Contents/MacOS/VideoDock"
            fi
        fi
    done
    macdeployqt "$@" "-codesign=$identity"
    # The first pass discovers QML plugins after collecting the initial framework graph.
    macdeployqt "$app" -always-overwrite -no-plugins "-codesign=$identity"
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
        codesign --force --sign "$identity" "$library"
    done
    python3 "$project/scripts/rust/repair-macos-frameworks.py" "$app" "$identity"
    codesign --verify --deep --strict "$app"
    "$app/Contents/MacOS/ffmpeg" -version >/dev/null
    "$app/Contents/MacOS/ffprobe" -version >/dev/null
    publish_install
    ;;
  *) echo 'Use build-windows.ps1 on Windows' >&2; exit 1 ;;
esac
printf '%s\n' 'VideoDock installed. FFmpeg/FFprobe must be available in PATH.'
