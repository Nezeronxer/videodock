#include "platform.h"
#include <QGuiApplication>
#include <QNetworkCookie>
#include <QQuickWebEngineProfile>
#include <QQmlApplicationEngine>
#include <QTemporaryDir>
#include <QDir>
#include <QStandardPaths>
#include <QDateTime>
#include <QTimer>
#include <QWebEngineCookieStore>
#include <cassert>
#include <cstdio>
#include <cstdint>

class Receiver final : public QObject {
    Q_OBJECT
public:
    bool added = false;
    bool removed = false;
    ::std::int64_t expectedExpiry = 0;
public slots:
    void cookieEvent(const QString &name, const QString &value, const QString &domain,
                     const QString &path, bool secure, ::std::int64_t expires, bool deleted) {
        if (name != QStringLiteral("videodock_bridge_test")) return;
        assert(domain.endsWith(QStringLiteral("localhost")));
        assert(path == QStringLiteral("/"));
        assert(!secure);
        assert(expires == expectedExpiry);
        if (deleted) {
            assert(value.isEmpty());
            removed = true;
            QCoreApplication::quit();
        } else {
            assert(value == QStringLiteral("synthetic_test_value"));
            added = true;
            clear_browser_cookies();
        }
    }
};

int main(int argc, char **argv) {
    initialize_webengine();
    QGuiApplication app(argc, argv);
    QCoreApplication::setApplicationName(QStringLiteral("videodock"));
    const auto data = application_data_directory();
    assert(QDir::isAbsolutePath(data));
    assert(data == QStandardPaths::writableLocation(QStandardPaths::AppLocalDataLocation));
    QTemporaryDir directory;
    assert(directory.isValid());
    prepare_profile(directory.path());
    Receiver receiver;
    watch_cookies(receiver);
    QQmlApplicationEngine engine;
    engine.loadData("import QtQuick\nimport QtWebEngine\nWindow { width:320; height:200; visible:true; WebEngineView { anchors.fill:parent; url:\"about:blank\" } }");
    assert(!engine.rootObjects().isEmpty());
    QNetworkCookie cookie("videodock_bridge_test", "synthetic_test_value");
    cookie.setDomain(QStringLiteral("localhost"));
    cookie.setPath(QStringLiteral("/"));
    receiver.expectedExpiry = QDateTime::currentSecsSinceEpoch() + 3600;
    cookie.setExpirationDate(QDateTime::fromSecsSinceEpoch(receiver.expectedExpiry));
    QTimer::singleShot(500, [&] {
        QQuickWebEngineProfile::defaultProfile()->cookieStore()->setCookie(cookie, QUrl(QStringLiteral("http://localhost")));
    });
    QTimer::singleShot(15000, &app, &QCoreApplication::quit);
    app.exec();
    assert(receiver.added && receiver.removed);
    std::puts("PASS: cookieAdded and cookieRemoved delivered directly to QObject; removed values empty.");
}

#include "cookie_bridge.moc"
