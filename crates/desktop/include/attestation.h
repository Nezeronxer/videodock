#pragma once
#include <QUrl>
#include <QByteArray>
class QIODevice;
class QNetworkRequest;

// Shared by the worker and its offline trust-boundary checks.
bool proof_interpreter_url(const QUrl &url);
bool proof_network_url(const QUrl &url);
bool proof_read_interpreter(QIODevice &input, QByteArray &body);
QNetworkRequest proof_interpreter_request(const QUrl &source, const QString &google_address);
