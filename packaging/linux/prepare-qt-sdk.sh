#!/bin/sh
set -eu
# CXX-Qt searches one Qt prefix; KDE SDK and WebEngine BaseApp use two.
for kind in lib include; do
    mkdir -p "/run/videodock-target/qt-merge/$kind"
    if [ "$kind" = lib ]; then
        roots='/app/lib/x86_64-linux-gnu /app/lib /usr/lib/x86_64-linux-gnu'
    else
        roots='/app/include /usr/include'
    fi
    for root in $roots; do
        for entry in "$root"/*; do
            dest="/run/videodock-target/qt-merge/$kind/$(basename "$entry")"
            if [ ! -e "$dest" ] && [ ! -L "$dest" ]; then ln -s "$entry" "$dest"; fi
        done
    done
done
version=$(/usr/bin/qmake -query QT_VERSION)
for module in Core Quick WebEngineCore WebEngineQuick WebEngineWidgets; do
    file="/run/videodock-target/qt-merge/lib/libQt6$module.prl"
    test -f "$file"
    grep -Fqx "QMAKE_PRL_VERSION = $version" "$file"
done
