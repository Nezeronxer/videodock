#pragma once
#include <QObject>
#include <QString>
#include <QGuiApplication>
#include <memory>
void initialize_webengine();
std::unique_ptr<QGuiApplication> create_application();
QString application_data_directory();
void prepare_profile(const QString &directory);
void watch_cookies(QObject &backend);
void clear_browser_cookies();
void install_translation(const QString &language);
