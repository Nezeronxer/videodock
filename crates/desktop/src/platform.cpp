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

bool configure_browser_proxy(const QString &address) {
    QNetworkProxy proxy(QNetworkProxy::NoProxy);
    if (!address.isEmpty()) {
        const QUrl url(address);
        const auto scheme = url.scheme();
        const bool socks = scheme == QStringLiteral("socks5h") || scheme == QStringLiteral("socks5");
        if (!url.isValid() || url.host().isEmpty() || url.port() <= 0
            || (scheme != QStringLiteral("http") && !socks)
            || (socks && !url.userName().isEmpty())) {
            // WebEngine lacks HTTPS/SOCKS4 proxies and SOCKS authentication; never fall back to direct.
            QNetworkProxy::setApplicationProxy(QNetworkProxy(QNetworkProxy::HttpProxy,
                                                             QStringLiteral("127.0.0.1"), 9));
            return false;
        }
        proxy = QNetworkProxy(socks ? QNetworkProxy::Socks5Proxy : QNetworkProxy::HttpProxy,
                              url.host(), quint16(url.port()), url.userName(), url.password());
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
        if (qtTranslator.load(QStringLiteral("qtbase_ru"), directory))
            QCoreApplication::installTranslator(&qtTranslator);
        if (declarativeTranslator.load(QStringLiteral("qtdeclarative_ru"), directory))
            QCoreApplication::installTranslator(&declarativeTranslator);
    }
    if (language.startsWith(QStringLiteral("ru")) && translator.load(QStringLiteral(":/translations/videodock_ru.qm")))
        QCoreApplication::installTranslator(&translator);
}
