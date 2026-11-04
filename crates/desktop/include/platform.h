#pragma once
#include <QObject>
#include <QString>
#include <QGuiApplication>
#include <memory>
void initialize_webengine();
std::unique_ptr<QGuiApplication> create_application();
QString application_data_directory();
void prepare_profile(const QString &directory);
bool configure_browser_proxy(const QString &address);
void watch_cookies(QObject &backend);
void clear_browser_cookies();
void install_translation(const QString &language);
QString reference_asset(const QString &name);
