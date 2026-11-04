#include "platform.h"
#include <QGuiApplication>
#include <QNetworkProxy>
#include <QQmlApplicationEngine>
#include <QQmlContext>
#include <QTcpServer>
#include <QTcpSocket>
#include <QTimer>
#include <QTemporaryDir>
#include <QEventLoop>
#include <cassert>
#include <cstdio>
#include <cstdlib>

class Probe final : public QObject {
    Q_OBJECT
public:
    QEventLoop loop;
    bool loaded = false;
public slots:
    void finish(bool success) { loaded = success; loop.quit(); }
};

int main(int argc, char **argv) {
    initialize_webengine();
    QGuiApplication app(argc, argv);
    QTemporaryDir directory;
    assert(directory.isValid());
    const int selected = argc > 1 ? std::atoi(argv[1]) : 0;
    for (int mode = selected; mode <= selected; ++mode) {
        const bool socks = mode == 2;
        const bool auth = mode == 1;
        QTcpServer server;
        assert(server.listen(QHostAddress::LocalHost));
        int requests = 0;
        int authorized = 0;
        QObject::connect(&server, &QTcpServer::newConnection, [&] {
            auto *socket = server.nextPendingConnection();
            socket->setProperty("phase", socks ? 0 : 2);
            QObject::connect(socket, &QTcpSocket::readyRead, [&, socket] {
                auto bytes = socket->property("buffer").toByteArray() + socket->readAll();
                auto phase = socket->property("phase").toInt();
                if (phase == 0) {
                    if (bytes.size() < 2 || bytes.size() < 2 + quint8(bytes[1])) {
                        socket->setProperty("buffer", bytes); return;
                    }
                    assert(quint8(bytes[0]) == 5);
                    bytes.remove(0, 2 + quint8(bytes[1]));
                    socket->write(QByteArray::fromHex("0500"));
                    phase = 1;
                }
                if (phase == 1) {
                    if (bytes.size() < 5) { socket->setProperty("phase", phase); socket->setProperty("buffer", bytes); return; }
                    assert(quint8(bytes[0]) == 5 && quint8(bytes[1]) == 1);
                    const int addressSize = quint8(bytes[3]) == 3 ? 1 + quint8(bytes[4]) : 4;
                    const int size = 4 + addressSize + 2;
                    if (bytes.size() < size) { socket->setProperty("phase", phase); socket->setProperty("buffer", bytes); return; }
                    bytes.remove(0, size);
                    socket->write(QByteArray::fromHex("050000017f0000010000"));
                    phase = 2;
                }
                socket->setProperty("phase", phase);
                socket->setProperty("buffer", bytes);
                if (phase != 2 || !bytes.contains("\r\n\r\n")) return;
                ++requests;
                const bool valid = bytes.toLower().contains("proxy-authorization: basic dgvzddpzew50agv0awm=");
                if (auth && !valid) {
                    socket->write("HTTP/1.1 407 Proxy Authentication Required\r\nProxy-Authenticate: Basic realm=\"synthetic\"\r\nContent-Length: 0\r\nConnection: close\r\n\r\n");
                } else {
                    if (valid) ++authorized;
                    socket->write("HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: 17\r\nConnection: close\r\n\r\n<title>ok</title>");
                }
                socket->disconnectFromHost();
                socket->setProperty("phase", 3);
            });
            QObject::connect(socket, &QTcpSocket::disconnected, socket, &QObject::deleteLater);
        });
        const auto address = QStringLiteral("%1://%2%3:%4")
            .arg(socks ? QStringLiteral("socks5h") : QStringLiteral("http"),
                 auth ? QStringLiteral("test:synthetic@") : QString(), QStringLiteral("127.0.0.1"))
            .arg(server.serverPort());
        assert(configure_browser_proxy(address));
        prepare_profile(directory.path());
        const auto configured = QNetworkProxy::applicationProxy();
        assert(configured.hostName() == QStringLiteral("127.0.0.1"));
        assert(configured.type() == (socks ? QNetworkProxy::Socks5Proxy : QNetworkProxy::HttpProxy));
        Probe probe;
        QQmlApplicationEngine engine;
        engine.rootContext()->setContextProperty(QStringLiteral("probe"), &probe);
        engine.loadData(QStringLiteral(
            "import QtQuick\nimport QtWebEngine\nWindow { width:320; height:200; visible:true; WebEngineView { anchors.fill:parent; url: \"http://proxy-routing-%1.invalid/probe\"; onLoadingChanged:function(request) { if(request.status===WebEngineView.LoadSucceededStatus) probe.finish(true); else if(request.status===WebEngineView.LoadFailedStatus) probe.finish(false); } } }").arg(mode).toUtf8());
        QTimer::singleShot(10000, &probe.loop, &QEventLoop::quit);
        probe.loop.exec();
        assert(probe.loaded && requests > 0);
        if (auth) assert(authorized > 0);
        std::printf("PASS: WebEngine %s auth=%s requests=%d authorized=%d\n", socks ? "SOCKS5" : "HTTP", auth ? "yes" : "no", requests, authorized);
    }
    for (const auto &address : {QStringLiteral("socks5h://test:synthetic@127.0.0.1:1080"),
                                 QStringLiteral("https://127.0.0.1:1080"), QStringLiteral("socks4://127.0.0.1:1080")}) {
        assert(!configure_browser_proxy(address));
        assert(QNetworkProxy::applicationProxy().type() != QNetworkProxy::NoProxy);
    }
    assert(configure_browser_proxy(QString()));
    assert(QNetworkProxy::applicationProxy().type() == QNetworkProxy::NoProxy);
    std::puts("PASS: unsupported browser proxy configurations fail closed; disabled proxy becomes explicit direct.");
}
#include "browser_proxy.moc"
