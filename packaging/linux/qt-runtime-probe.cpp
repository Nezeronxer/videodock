// Build-time verification only; this executable is never included in the release artifact.
#include <QApplication>
#include <QWebEngineView>
#include <QWebEnginePage>
#include <QWebEngineProfile>
#include <QWebEngineSettings>
#include <QNetworkProxy>
#include <QTimer>
#include <QFile>
#include <QTextStream>
#include <QJsonDocument>
#include <QJsonObject>
int main(int argc,char**argv){
 QCoreApplication::setAttribute(Qt::AA_ShareOpenGLContexts);QApplication app(argc,argv);
 qunsetenv("QTWEBENGINE_DISABLE_SANDBOX");qputenv("QTWEBENGINE_CHROMIUM_FLAGS","--disable-quic");
 QNetworkProxy::setApplicationProxy(QNetworkProxy(QNetworkProxy::Socks5Proxy,"127.0.0.1",1080));
 QWebEngineProfile profile;profile.setHttpCacheType(QWebEngineProfile::MemoryHttpCache);profile.setPersistentCookiesPolicy(QWebEngineProfile::NoPersistentCookies);
 QWebEnginePage page(&profile);QWebEngineView view;view.setPage(&page);view.setAttribute(Qt::WA_ShowWithoutActivating);view.resize(800,600);view.setWindowTitle("VideoDock packaging verification");view.show();
 QObject::connect(&page,&QWebEnginePage::loadFinished,&app,[&](bool ok){if(!ok){QTextStream(stdout)<<"page_load=false"<<Qt::endl;app.exit(2);return;}
  page.runJavaScript("JSON.stringify({canvas:!!document.createElement('canvas').getContext('2d'),webdriver:navigator.webdriver})",[&](const QVariant&result){const auto obj=QJsonDocument::fromJson(result.toString().toUtf8()).object();QFile args(QStringLiteral("/proc/%1/cmdline").arg(page.renderProcessPid()));args.open(QIODevice::ReadOnly);const auto flags=args.readAll();const bool nativeSandbox=!flags.isEmpty()&&!flags.contains("--no-sandbox");QTextStream(stdout)<<"page_load=true off_record="<<profile.isOffTheRecord()<<" canvas="<<obj.value("canvas").toBool()<<" webdriver="<<obj.value("webdriver").toBool()<<" renderer_observed="<<!flags.isEmpty()<<" no_sandbox_flag="<<flags.contains("--no-sandbox")<<Qt::endl;app.exit(nativeSandbox&&obj.value("canvas").toBool()?0:3);});});
 QTimer::singleShot(45000,&app,[&]{QTextStream(stdout)<<"runtime_probe_timeout=true"<<Qt::endl;app.exit(4);});page.load(QUrl("https://www.youtube.com/"));return app.exec();
}
