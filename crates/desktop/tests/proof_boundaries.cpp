#include "attestation.h"
#include <QCoreApplication>
#include <QBuffer>
#include <QNetworkRequest>
#include <QUrl>
#include <cassert>
#include <cstdio>

int main(int argc, char **argv) {
    QCoreApplication application(argc, argv);
    assert(proof_interpreter_url(QUrl("https://www.google.com/js/th/fixture_123-ABC.js")));
    const auto source = QUrl("https://www.google.com/js/th/fixture.js");
    const auto direct = proof_interpreter_request(source, QStringLiteral("142.250.74.206"));
    assert(direct.url().host() == QStringLiteral("142.250.74.206"));
    assert(direct.rawHeader("Host") == "www.google.com");
    assert(direct.peerVerifyName() == QStringLiteral("www.google.com"));
    assert(!direct.attribute(QNetworkRequest::Http2AllowedAttribute).toBool());
    assert(direct.attribute(QNetworkRequest::RedirectPolicyAttribute).toInt() == QNetworkRequest::ManualRedirectPolicy);
    const auto unchanged = proof_interpreter_request(source, QString());
    assert(unchanged.url() == source && unchanged.peerVerifyName().isEmpty());
    assert(proof_interpreter_request(source, QStringLiteral("not-an-ip")).url().isEmpty());
    for (const auto *url : {
        "http://www.google.com/js/th/fixture.js", "https://www.google.com:444/js/th/fixture.js",
        "https://user:private@www.google.com/js/th/fixture.js", "https://google.com/js/th/fixture.js",
        "https://www.google.com.evil.invalid/js/th/fixture.js", "https://www.google.com/js/th/../fixture.js",
        "https://www.google.com/js/th/fixture.js?redirect=1", "https://www.google.com/js/th/fixture.js#fragment",
        "https://www.google.com/accounts/fixture.js", "https://127.0.0.1/js/th/fixture.js"}) {
        assert(!proof_interpreter_url(QUrl(url)));
    }
    assert(proof_network_url(QUrl("https://www.youtube.com/")));
    assert(proof_network_url(QUrl("https://i.ytimg.com/image.jpg")));
    for (const auto *url : {
        "https://accounts.google.com/", "https://accounts.youtube.com/", "https://r1.googlevideo.com/videoplayback",
        "https://googlevideo.com/", "https://youtube.com.evil.invalid/", "http://www.youtube.com/",
        "https://127.0.0.1/", "https://www.youtube.com:8443/", "file:///etc/passwd"}) {
        assert(!proof_network_url(QUrl(url)));
    }
    QByteArray oversized(4 * 1024 * 1024 + 1, 'x');
    QBuffer input(&oversized);
    assert(input.open(QIODevice::ReadOnly));
    QByteArray body;
    assert(!proof_read_interpreter(input, body));
    assert(body.isEmpty());
    oversized.resize(4 * 1024 * 1024);
    input.seek(0);
    assert(proof_read_interpreter(input, body));
    assert(body.size() == 4 * 1024 * 1024);
    QByteArray extra("x");
    QBuffer trailing(&extra);
    assert(trailing.open(QIODevice::ReadOnly));
    assert(!proof_read_interpreter(trailing, body));
    assert(body.size() == 4 * 1024 * 1024);
    std::puts("proof URL/redirect boundaries: passed");
}
