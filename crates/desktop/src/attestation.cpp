#include "platform.h"
#include "attestation.h"
#include <QByteArray>
#include <QAuthenticator>
#include <QCoreApplication>
#include <QEventLoop>
#include <QFile>
#include <QJsonDocument>
#include <QJsonObject>
#include <QHostAddress>
#include <QNetworkAccessManager>
#include <QNetworkProxy>
#include <QNetworkReply>
#include <QNetworkRequest>
#include <QRegularExpression>
#include <QTimer>
#include <QWebEngineDownloadRequest>
#include <QWebEnginePage>
#include <QWebEnginePermission>
#include <QWebEngineProfile>
#include <QWebEngineSettings>
#include <QWebEngineUrlRequestInfo>
#include <QWebEngineUrlRequestInterceptor>
#include <QWebEngineView>
#include <cstdio>
#include <functional>
#include <memory>

namespace {
constexpr qsizetype interpreter_limit = 4 * 1024 * 1024;
bool https(const QUrl &url) {
    return url.isValid() && url.scheme() == QStringLiteral("https")
        && !url.host().isEmpty() && url.port(443) == 443
        && url.userName().isEmpty() && url.password().isEmpty();
}
QString encoded(const QJsonObject &value) {
    return QString::fromUtf8(QJsonDocument(value).toJson(QJsonDocument::Compact));
}
class Guard final : public QWebEngineUrlRequestInterceptor {
public:
    void interceptRequest(QWebEngineUrlRequestInfo &request) override {
        if (!proof_network_url(request.requestUrl())) request.block(true);
    }
};
class Page final : public QWebEnginePage {
public:
    using QWebEnginePage::QWebEnginePage;
    void javaScriptConsoleMessage(JavaScriptConsoleMessageLevel, const QString &, int, const QString &) override {}
    bool acceptNavigationRequest(const QUrl &url, NavigationType, bool main_frame) override {
        return proof_network_url(url) && (!main_frame
            || (url.host() == QStringLiteral("www.youtube.com") && url.path() == QStringLiteral("/")
                && !url.hasQuery() && !url.hasFragment()));
    }
};
}

bool proof_interpreter_url(const QUrl &url) {
    static const QRegularExpression path(QStringLiteral("^/js/th/[A-Za-z0-9_-]+\\.js$"));
    return https(url) && url.host() == QStringLiteral("www.google.com")
        && !url.hasQuery() && !url.hasFragment() && path.match(url.path()).hasMatch();
}
bool proof_network_url(const QUrl &url) {
    if (!https(url)) return false;
    const auto host = url.host().toLower();
    if (host.startsWith(QStringLiteral("accounts."))
        || host == QStringLiteral("googlevideo.com") || host.endsWith(QStringLiteral(".googlevideo.com"))) return false;
    for (const auto *domain : {"youtube.com", "google.com", "ytimg.com", "gstatic.com", "googleapis.com", "googleusercontent.com"}) {
        const auto suffix = QString::fromLatin1(domain);
        if (host == suffix || host.endsWith(QStringLiteral(".") + suffix)) return true;
    }
    return false;
}

bool proof_read_interpreter(QIODevice &input, QByteArray &body) {
    const auto remaining = interpreter_limit - body.size();
    if (remaining < 0 || input.bytesAvailable() > remaining) return false;
    body.append(input.read(remaining));
    return body.size() <= interpreter_limit;
}

QNetworkRequest proof_interpreter_request(const QUrl &source, const QString &google_address) {
    if (!proof_interpreter_url(source)) return {};
    QNetworkRequest request(source);
    if (!google_address.isEmpty()) {
        const QHostAddress address(google_address);
        if (address.protocol() != QAbstractSocket::IPv4Protocol) return {};
        auto destination = source;
        destination.setHost(address.toString());
        request.setUrl(destination);
        request.setRawHeader("Host", source.host().toLatin1());
        request.setPeerVerifyName(source.host());
    }
    request.setAttribute(QNetworkRequest::RedirectPolicyAttribute, QNetworkRequest::ManualRedirectPolicy);
    // HTTP/2 constructs :authority from the dial IP; HTTP/1.1 preserves our original Host.
    request.setAttribute(QNetworkRequest::Http2AllowedAttribute, false);
    request.setTransferTimeout(30000);
    return request;
}

void initialize_proof_runtime() {
    // The child owns these settings; they never alter the running application's browser.
    qunsetenv("QTWEBENGINE_REMOTE_DEBUGGING");
    qunsetenv("QTWEBENGINE_DISABLE_SANDBOX");
    qputenv("QTWEBENGINE_CHROMIUM_FLAGS", "--disable-quic --disable-background-networking --force-webrtc-ip-handling-policy=disable_non_proxied_udp");
    qInstallMessageHandler([](QtMsgType, const QMessageLogContext &, const QString &) {});
#ifdef Q_OS_WIN
    std::freopen("NUL", "w", stderr);
#else
    std::freopen("/dev/null", "w", stderr);
#endif
}

QString run_youtube_proof(const QString &video_id, const QString &proxy, const QString &google_address) {
    if (!proxy.isEmpty() && !google_address.isEmpty()) return encoded({{"ok", false}, {"stage", "validation"}});
    if (!configure_browser_proxy(proxy)) return encoded({{"ok", false}, {"stage", "network"}});
    auto selected_proxy = QNetworkProxy::applicationProxy();
    const QUrl proxy_url(proxy);
    selected_proxy.setUser(proxy_url.userName(QUrl::FullyDecoded));
    selected_proxy.setPassword(proxy_url.password(QUrl::FullyDecoded));
    QNetworkProxy::setApplicationProxy(selected_proxy);
    QString result = encoded({{"ok", false}, {"stage", "internal"}});
    bool finished = false;
    QEventLoop loop;
    QWebEngineProfile profile;
    profile.setPersistentCookiesPolicy(QWebEngineProfile::NoPersistentCookies);
    profile.setHttpCacheType(QWebEngineProfile::MemoryHttpCache);
    Guard guard;
    profile.setUrlRequestInterceptor(&guard);
    Page page(&profile);
    QWebEngineView view;
    view.setPage(&page);
    view.resize(960, 640);
    view.setAttribute(Qt::WA_DontShowOnScreen);
    page.settings()->setAttribute(QWebEngineSettings::PlaybackRequiresUserGesture, true);
    page.settings()->setAttribute(QWebEngineSettings::WebRTCPublicInterfacesOnly, true);
    page.settings()->setAttribute(QWebEngineSettings::LocalContentCanAccessRemoteUrls, false);
    QNetworkAccessManager network;
    auto finish = [&](const QJsonObject &value) {
        if (finished) return;
        finished = true;
        result = encoded(value);
        page.triggerAction(QWebEnginePage::Stop);
        view.hide();
        loop.quit();
    };
    auto fail = [&](const char *stage) { finish({{"ok", false}, {"stage", QString::fromLatin1(stage)}}); };

    network.setProxy(QNetworkProxy::applicationProxy());
    if (!profile.isOffTheRecord()) return result;
    QFile script(QStringLiteral(":/videodock/proof/attest.js"));
    if (!script.open(QIODevice::ReadOnly)) return result;
    const auto javascript = QString::fromUtf8(script.readAll());
    bool started = false;
    bool interpreter_pending = false;
    std::function<void(QUrl, int)> fetch_interpreter;
    QTimer poll;
    int browser_auth_attempts = 0;
    int network_auth_attempts = 0;
    // Disconnect every signal/timer before its captured stack state is destroyed.
    auto matches_proxy = [&](const QNetworkProxy &candidate) {
        return candidate.type() == QNetworkProxy::HttpProxy && selected_proxy.type() == QNetworkProxy::HttpProxy
            && candidate.hostName().compare(selected_proxy.hostName(), Qt::CaseInsensitive) == 0
            && candidate.port() == selected_proxy.port();
    };
    QObject callbacks;
    QObject::connect(&page, &QWebEnginePage::proxyAuthenticationRequired, &callbacks,
        [&](const QUrl &, QAuthenticator *authenticator, const QString &host) {
            if (!matches_proxy(QNetworkProxy::applicationProxy())
                || host.compare(selected_proxy.hostName(), Qt::CaseInsensitive) != 0
                || ++browser_auth_attempts > 8) { fail("network"); return; }
            authenticator->setUser(selected_proxy.user());
            authenticator->setPassword(selected_proxy.password());
        });
    QObject::connect(&network, &QNetworkAccessManager::proxyAuthenticationRequired, &callbacks,
        [&](const QNetworkProxy &candidate, QAuthenticator *authenticator) {
            if (!matches_proxy(candidate) || ++network_auth_attempts > 2) { fail("network"); return; }
            authenticator->setUser(selected_proxy.user());
            authenticator->setPassword(selected_proxy.password());
        });
    QObject::connect(&page, &QWebEnginePage::authenticationRequired, &callbacks,
        [](const QUrl &, QAuthenticator *authenticator) { *authenticator = QAuthenticator(); });
    QObject::connect(&network, &QNetworkAccessManager::authenticationRequired, &callbacks,
        [](QNetworkReply *, QAuthenticator *authenticator) { *authenticator = QAuthenticator(); });
    QObject::connect(&profile, &QWebEngineProfile::downloadRequested, &callbacks,
        [](QWebEngineDownloadRequest *request) { request->cancel(); });
    QObject::connect(&page, &QWebEnginePage::permissionRequested, &callbacks,
        [](QWebEnginePermission permission) { permission.deny(); });
    QObject::connect(&page, &QWebEnginePage::renderProcessTerminated, &callbacks,
        [&](QWebEnginePage::RenderProcessTerminationStatus, int) { fail("internal"); });
    fetch_interpreter = [&](QUrl url, int redirects) {
        if (finished) return;
        if (!proof_interpreter_url(url) || redirects > 3) { fail("interpreter"); return; }
        const auto request = proof_interpreter_request(url, google_address);
        if (request.url().isEmpty()) { fail("interpreter"); return; }
        auto *reply = network.get(request);
        auto body = std::make_shared<QByteArray>();
        QObject::connect(reply, &QNetworkReply::metaDataChanged, &callbacks, [&, reply] {
            if (reply->header(QNetworkRequest::ContentLengthHeader).toLongLong() > interpreter_limit) {
                fail("interpreter"); reply->abort();
            }
        });
        auto consume = [&, reply, body] {
            if (finished) return;
            if (!proof_read_interpreter(*reply, *body)) {
                fail("interpreter"); reply->abort(); return;
            }
        };
        QObject::connect(reply, &QNetworkReply::readyRead, &callbacks, consume);
        QObject::connect(reply, &QNetworkReply::finished, &callbacks, [&, reply, body, consume, url, redirects] {
            reply->deleteLater();
            if (finished) return;
            consume();
            if (finished) return;
            const auto code = reply->attribute(QNetworkRequest::HttpStatusCodeAttribute).toInt();
            if (code >= 300 && code <= 399) {
                const auto next = url.resolved(reply->attribute(QNetworkRequest::RedirectionTargetAttribute).toUrl());
                fetch_interpreter(next, redirects + 1);
                return;
            }
            if (reply->error() != QNetworkReply::NoError || code != 200 || body->isEmpty()) { fail("interpreter"); return; }
            page.runJavaScript(QString::fromUtf8(*body), [&](const QVariant &) {
                if (!finished) page.runJavaScript(QStringLiteral("globalThis.__videodockInterpreterResolve();"));
            });
        });
    };

    QObject::connect(&page, &QWebEnginePage::loadFinished, &callbacks, [&](bool ok) {
        if (finished || started) return;
        started = true;
        if (!ok || page.url() != QUrl(QStringLiteral("https://www.youtube.com/"))) { fail("homepage"); return; }
        page.runJavaScript(javascript + QStringLiteral("\nvideodockAttest('%1');").arg(video_id));
    });
    QObject::connect(&poll, &QTimer::timeout, &callbacks, [&] {
        if (finished || !started) return;
        page.runJavaScript(QString::fromLatin1(R"JS((() => {
            const s = globalThis.__videodockAttestation;
            if (!s || typeof s !== 'object') return '{"state":"pending"}';
            const out = {state:s.state};
            const copy = (key, limit) => {
                if (typeof s[key] !== 'string' || !s[key].length || s[key].length > limit) throw Error();
                out[key] = s[key];
            };
            try {
                if (s.state === 'interpreter') copy('url', 2048);
                else if (s.state === 'failed') copy('stage', 32);
                else if (s.state === 'ready') {
                    copy('content_po', 8192); copy('session_po', 8192);
                    copy('visitor_data', 4096); copy('user_agent', 512);
                } else return '{"state":"pending"}';
                return JSON.stringify(out);
            } catch { return '{"state":"failed","stage":"internal"}'; }
        })())JS"), [&](const QVariant &value) {
            if (finished) return;
            const auto bytes = value.toString().toUtf8();
            if (bytes.size() > 32 * 1024) { fail("internal"); return; }
            const auto object = QJsonDocument::fromJson(bytes).object();
            const auto state = object.value(QStringLiteral("state")).toString();
            if (state == QStringLiteral("interpreter") && !interpreter_pending) {
                interpreter_pending = true;
                fetch_interpreter(QUrl(object.value(QStringLiteral("url")).toString()), 0);
            } else if (state == QStringLiteral("failed")) {
                finish({{"ok", false}, {"stage", object.value(QStringLiteral("stage"))}});
            } else if (state == QStringLiteral("ready")) {
                finish({{"ok", true}, {"content_po", object.value(QStringLiteral("content_po"))},
                    {"session_po", object.value(QStringLiteral("session_po"))},
                    {"visitor_data", object.value(QStringLiteral("visitor_data"))},
                    {"user_agent", object.value(QStringLiteral("user_agent"))}});
            }
        });
    });
    poll.start(250);
    QTimer::singleShot(85000, &callbacks, [&] { fail("timeout"); });
    QTimer::singleShot(0, &callbacks, [&] { view.show(); page.load(QUrl(QStringLiteral("https://www.youtube.com/"))); });
    loop.exec();
    finished = true;
    return result;
}
