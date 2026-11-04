#pragma once
#include <QObject>
#include <QString>
#include <QGuiApplication>
#include <memory>
void initialize_webengine();
std::unique_ptr<QGuiApplication> create_application();
std::unique_ptr<QGuiApplication> create_proof_application(const QString &resolver_rules);
QString application_data_directory();
void prepare_profile(const QString &directory);
bool configure_browser_proxy(const QString &address);
void watch_cookies(QObject &backend);
void clear_browser_cookies();
void install_translation(const QString &language);
QString reference_asset(const QString &name);
void initialize_proof_runtime();
QString run_youtube_proof(const QString &video_id, const QString &proxy, const QString &google_address);
