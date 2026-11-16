#include "platform.h"
#include <QCoreApplication>
#include <QApplication>
#include <QDir>
#include <QStandardPaths>
#include <QDateTime>
#include <QResource>
#include <cstdint>
#include <QNetworkCookie>
#include <QNetworkProxy>
#include <QTcpServer>
#include <QTcpSocket>
#include <QTimer>
#include <QHostAddress>
#include <QUrl>
#include <QTranslator>
#include <QLibraryInfo>
#include <QWebEngineCookieStore>
#include <QQuickWebEngineProfile>
#include <QtWebEngineQuick/qtwebenginequickglobal.h>

void initialize_webengine() {
    QCoreApplication::setAttribute(Qt::AA_ShareOpenGLContexts);
    QtWebEngineQuick::initialize();
}

std::unique_ptr<QGuiApplication> create_application() {
    static int argc = 1;
    static char name[] = "videodock";
    static char *argv[] = {name, nullptr};
    auto application = std::make_unique<QApplication>(argc, argv);
    QCoreApplication::setApplicationName(QStringLiteral("videodock"));
    return application;
}

QString application_data_directory() {
    return QStandardPaths::writableLocation(QStandardPaths::AppLocalDataLocation);
}

std::unique_ptr<QGuiApplication> create_proof_application(const QString &resolver_rules) {
    static QByteArray name("videodock-proof");
    static QByteArray marker("--webEngineArgs");
    static QByteArray rules = QByteArray("--host-resolver-rules=") + resolver_rules.toUtf8();
    static char *argv[] = {name.data(), marker.data(), rules.data(), nullptr};
    static int argc = resolver_rules.isEmpty() ? 1 : 3;
    auto application = std::make_unique<QApplication>(argc, argv);
    QCoreApplication::setApplicationName(QStringLiteral("videodock-proof"));
    return application;
}

namespace {
// Chromium cannot authenticate SOCKS; Qt tunnels it without exposing upstream credentials to pages.
class SocksBridge final : public QTcpServer {
public:
    explicit SocksBridge(const QNetworkProxy &upstream) : QTcpServer(QCoreApplication::instance()) {
        setProxy(QNetworkProxy::NoProxy);
        connect(this, &QTcpServer::newConnection, this, [this, upstream] {
            while (hasPendingConnections()) {
                auto *client = nextPendingConnection();
                if (connections >= 32) { client->deleteLater(); continue; }
                ++connections;
                connect(client, &QObject::destroyed, this, [this] { --connections; });
                auto *remote = new QTcpSocket(client);
                remote->setProxy(upstream);
                client->setReadBufferSize(64 * 1024);
                remote->setReadBufferSize(64 * 1024);
                auto *timer = new QTimer(client);
                timer->setSingleShot(true);
                timer->start(30000);
                connect(timer, &QTimer::timeout, client, &QObject::deleteLater);
                connect(remote, &QTcpSocket::connected, client, [client, remote, timer] {
                    timer->stop();
                    client->setProperty("phase", 2);
                    client->write(QByteArray::fromHex("050000017f0000010000"));
                    remote->write(client->readAll());
                });
                connect(remote, &QTcpSocket::readyRead, client, [client, remote] {
                    if (client->bytesToWrite() < 256 * 1024) client->write(remote->readAll());
                });
                connect(client, &QTcpSocket::bytesWritten, remote, [client, remote] {
                    if (client->bytesToWrite() < 256 * 1024) client->write(remote->readAll());
                });
                connect(remote, &QTcpSocket::bytesWritten, client, [client, remote] {
                    if (remote->bytesToWrite() < 256 * 1024) remote->write(client->readAll());
                });
                connect(client, &QTcpSocket::disconnected, client, &QObject::deleteLater);
                connect(remote, &QTcpSocket::disconnected, client, &QTcpSocket::disconnectFromHost);
                connect(remote, &QTcpSocket::errorOccurred, client, [client] { client->disconnectFromHost(); });
                connect(client, &QTcpSocket::readyRead, client, [client, remote] {
                    int phase = client->property("phase").toInt();
                    if (phase == 2) {
                        if (remote->bytesToWrite() < 256 * 1024) remote->write(client->readAll());
                        return;
                    }
                    if (phase == 3) return;
                    auto bytes = client->peek(1024);
                    if (phase == 0) {
                        if (bytes.size() < 2 || bytes.size() < 2 + quint8(bytes[1])) return;
                        if (quint8(bytes[0]) != 5 || !bytes.mid(2, quint8(bytes[1])).contains(char(0))) { client->deleteLater(); return; }
                        client->read(2 + quint8(bytes[1]));
                        client->write(QByteArray::fromHex("0500"));
                        client->setProperty("phase", 1);
                        bytes = client->peek(1024);
                    }
                    if (bytes.size() < 5) return;
                    if (bytes.left(3) != QByteArray::fromHex("050100")) { client->deleteLater(); return; }
                    const auto kind = quint8(bytes[3]);
                    const int length = kind == 3 ? 1 + quint8(bytes[4]) : kind == 1 ? 4 : kind == 4 ? 16 : 0;
                    if (!length) { client->deleteLater(); return; }
                    if (bytes.size() < 6 + length) return;
                    QString host;
                    if (kind == 3) host = QString::fromLatin1(bytes.mid(5, length - 1));
                    else if (kind == 1) {
                        quint32 ip = 0;
                        for (int i = 4; i < 8; ++i) ip = (ip << 8) | quint8(bytes[i]);
                        host = QHostAddress(ip).toString();
                    } else {
                        Q_IPV6ADDR ip;
                        for (int i = 0; i < 16; ++i) ip[i] = quint8(bytes[4 + i]);
                        host = QHostAddress(ip).toString();
                    }
                    const quint16 port = (quint8(bytes[4 + length]) << 8) | quint8(bytes[5 + length]);
                    if (host.isEmpty() || !port || host.contains(QChar(0))) { client->deleteLater(); return; }
                    client->read(6 + length);
                    client->setProperty("phase", 3);
                    remote->connectToHost(host, port);
                });
            }
        });
    }
private:
    int connections = 0;
};
}

bool configure_browser_proxy(const QString &address) {
    QNetworkProxy proxy(QNetworkProxy::NoProxy);
    if (!address.isEmpty()) {
        const QUrl url(address);
        const auto scheme = url.scheme();
        const bool socks = scheme == QStringLiteral("socks5h") || scheme == QStringLiteral("socks5");
        const auto port = url.port(socks ? -1 : 80);
        if (!url.isValid() || url.host().isEmpty() || port <= 0
            || (scheme != QStringLiteral("http") && !socks)) {
            // Unsupported transports never fall back to a direct connection.
            QNetworkProxy::setApplicationProxy(QNetworkProxy(QNetworkProxy::HttpProxy,
                                                             QStringLiteral("127.0.0.1"), 9));
            return false;
        }
        proxy = QNetworkProxy(socks ? QNetworkProxy::Socks5Proxy : QNetworkProxy::HttpProxy,
                              url.host(), quint16(port), url.userName(QUrl::FullyDecoded), url.password(QUrl::FullyDecoded));
        if (socks && (!url.userName().isEmpty() || !url.password().isEmpty())) {
            auto *bridge = new SocksBridge(proxy);
            if (!bridge->listen(QHostAddress::LocalHost)) {
                delete bridge;
                QNetworkProxy::setApplicationProxy(QNetworkProxy(QNetworkProxy::HttpProxy, QStringLiteral("127.0.0.1"), 9));
                return false;
            }
            proxy = QNetworkProxy(QNetworkProxy::Socks5Proxy, QStringLiteral("127.0.0.1"), bridge->serverPort());
        }
    }
    QNetworkProxy::setApplicationProxy(proxy);
    return true;
}

void prepare_profile(const QString &directory) {
    QResource::registerResource(QStandardPaths::writableLocation(QStandardPaths::GenericDataLocation)
                                + QStringLiteral("/videodock/appearance.rcc"));
    auto *profile = QQuickWebEngineProfile::defaultProfile();
    profile->setStorageName(QStringLiteral("VideoDock"));
    profile->setPersistentStoragePath(directory);
    profile->setCachePath(directory + QStringLiteral("/cache"));
    profile->setOffTheRecord(false);
    profile->setPersistentCookiesPolicy(QQuickWebEngineProfile::ForcePersistentCookies);
}

QString reference_asset(const QString &name) {
    if (name.contains('/') || name.contains('\\'))
        return {};
    const auto path = QStringLiteral(":/videodock/reference/") + name;
    return QResource(path).isValid() ? QStringLiteral("qrc") + path : QString();
}

void watch_cookies(QObject &backend) {
    auto *store = QQuickWebEngineProfile::defaultProfile()->cookieStore();
    auto deliver = [&backend](const QNetworkCookie &cookie, bool removed) {
        const auto name = QString::fromUtf8(cookie.name());
        const auto value = removed ? QString() : QString::fromUtf8(cookie.value());
        const auto domain = cookie.domain();
        const auto path = cookie.path();
        const auto secure = cookie.isSecure();
        const ::std::int64_t expires = cookie.isSessionCookie() ? -1 : qMax(qint64(0), cookie.expirationDate().toSecsSinceEpoch());
        QMetaObject::invokeMethod(&backend, "cookieEvent", Qt::DirectConnection,
                                  Q_ARG(QString, name), Q_ARG(QString, value),
                                  Q_ARG(QString, domain), Q_ARG(QString, path),
                                  Q_ARG(bool, secure), Q_ARG(::std::int64_t, expires), Q_ARG(bool, removed));
    };
    QObject::connect(store, &QWebEngineCookieStore::cookieAdded, &backend,
                     [deliver](const QNetworkCookie &cookie) { deliver(cookie, false); });
    QObject::connect(store, &QWebEngineCookieStore::cookieRemoved, &backend,
                     [deliver](const QNetworkCookie &cookie) { deliver(cookie, true); });
    store->loadAllCookies();
}

void clear_browser_cookies() {
    QQuickWebEngineProfile::defaultProfile()->cookieStore()->deleteAllCookies();
}

void install_translation(const QString &language) {
    static QTranslator qtTranslator;
    static QTranslator declarativeTranslator;
    static QTranslator translator;
    if (language.startsWith(QStringLiteral("ru"))) {
        const auto directory = QLibraryInfo::path(QLibraryInfo::TranslationsPath);
        if (qtTranslator.load(QStringLiteral("qt_ru"), directory) || qtTranslator.load(QStringLiteral("qtbase_ru"), directory))
            QCoreApplication::installTranslator(&qtTranslator);
        if (declarativeTranslator.load(QStringLiteral("qtdeclarative_ru"), directory))
            QCoreApplication::installTranslator(&declarativeTranslator);
    }
    if (language.startsWith(QStringLiteral("ru")) && translator.load(QStringLiteral(":/translations/videodock_ru.qm")))
        QCoreApplication::installTranslator(&translator);
}
