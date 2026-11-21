#pragma once

#include <QClipboard>
#include <QGuiApplication>
#include <QUrl>

inline QString clipboard_host() {
    const QUrl url(QGuiApplication::clipboard()->text().trimmed(), QUrl::StrictMode);
    if (!url.isValid() || (url.scheme() != "https" && url.scheme() != "http")
        || url.authority().contains('@')) return {};
    auto host = url.host().toLower();
    if (host.endsWith('.')) host.chop(1);
    return host;
}

inline void watch_clipboard(QObject &backend) {
    QObject::connect(QGuiApplication::clipboard(), &QClipboard::dataChanged, &backend, [&backend] {
        QMetaObject::invokeMethod(&backend, "clipboardEvent");
    });
}
