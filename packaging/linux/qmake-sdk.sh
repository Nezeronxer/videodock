#!/bin/sh
set -eu
if [ "$#" -eq 2 ] && [ "$1" = -query ]; then
    case "$2" in
      QT_INSTALL_LIBS) echo /run/videodock-target/qt-merge/lib; exit 0 ;;
      QT_INSTALL_HEADERS) echo /run/videodock-target/qt-merge/include; exit 0 ;;
    esac
fi
exec /usr/bin/qmake "$@"
