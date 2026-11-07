// Isolated frontend fixture: exercises the shipped QML without accounts or network requests.
#include <QApplication>
#include <QResource>
#include <QClipboard>
#include <QDir>
#include <QImage>
#include <QDialogButtonBox>
#include <QJsonArray>
#include <QJsonDocument>
#include <QJsonObject>
#include <QProcess>
#include <QProcessEnvironment>
#include <QTemporaryDir>
#include <QTcpServer>
#include <QTcpSocket>
#include <QElapsedTimer>
#include <QQmlApplicationEngine>
#include <QQmlContext>
#include <QQmlError>
#include <QQuickItem>
#include <QQuickStyle>
#include <QQuickWindow>
#include <QKeySequence>
#include <QTest>
#include <QScopeGuard>
#include <QTranslator>
#include <QLibraryInfo>
#include <QtWebEngineQuick/qtwebenginequickglobal.h>
#include <cmath>
#include <functional>
#include <stdexcept>

class FixtureBackend : public QObject {
    Q_OBJECT
    Q_PROPERTY(QString snapshot READ snapshot NOTIFY snapshotChanged)
public:
    using QObject::QObject;
    QJsonObject state{
        {"settings", QJsonObject{{"theme", "dark"}, {"smartMode", true}, {"quality", 1080}, {"mediaType", "video"}, {"container", "mp4"}, {"path", "/tmp/VideoDock"}}},
        {"onboarding_completed", true}, {"jobs", QJsonArray{}}, {"subscriptions", QJsonArray{}}, {"account", QJsonObject{}},
        {"capabilities", QJsonObject{{"tray", false}, {"auto_start", false}, {"prevent_sleep", false}}}
    };
    QJsonObject lastAction;
    QProcess engineProcess;
    QJsonObject exchange(const QJsonObject &action) {
        engineProcess.write(QJsonDocument(action).toJson(QJsonDocument::Compact) + '\n');
        checkProcess(engineProcess.waitForBytesWritten(5000), "Engine input timed out");
        while (!engineProcess.canReadLine())
            checkProcess(engineProcess.waitForReadyRead(5000), "Engine output timed out");
        const auto reply = QJsonDocument::fromJson(engineProcess.readLine()).object();
        checkProcess(reply["snapshot"].isObject(), "Engine did not return a snapshot");
        const auto next = reply["snapshot"].toObject();
        if (next != state) {
            state = next;
            emit snapshotChanged();
        }
        return reply["result"].toObject();
    }
    void startEngine(const QString &executable, const QString &directory) {
        auto environment = QProcessEnvironment::systemEnvironment();
        environment.insert("HOME", directory);
        environment.insert("USERPROFILE", directory);
        engineProcess.setProcessEnvironment(environment);
        engineProcess.start(executable, {QDir(directory).filePath("state")});
        checkProcess(engineProcess.waitForStarted(5000), "Engine process failed to start");
        exchange(QJsonObject{{"action", "app"}, {"command", "onboarding_complete"}});
    }
    void stopEngine() {
        engineProcess.closeWriteChannel();
        checkProcess(engineProcess.waitForFinished(5000), "Engine shutdown timed out");
        checkProcess(engineProcess.exitCode() == 0, "Engine exited with an error");
    }
    static void checkProcess(bool condition, const char *message) {
        if (!condition) throw std::runtime_error(message);
    }
    QString snapshot() const { return QString::fromUtf8(QJsonDocument(state).toJson(QJsonDocument::Compact)); }
    Q_INVOKABLE void refresh() {
        if (engineProcess.state() == QProcess::Running) exchange(QJsonObject{{"snapshot", true}});
    }
    Q_INVOKABLE void initializeProfile() { emit snapshotChanged(); }
    Q_INVOKABLE QString referenceAsset(const QString &name) {
        if (name.contains('/') || name.contains('\\')) return {};
        const auto path = QStringLiteral(":/videodock/reference/") + name;
        return QResource(path).isValid() ? QStringLiteral("qrc") + path : QString();
    }
    Q_INVOKABLE QString dispatch(const QString &text) {
        const auto action = QJsonDocument::fromJson(text.toUtf8()).object();
        lastAction = action;
        if (engineProcess.state() == QProcess::Running)
            return QString::fromUtf8(QJsonDocument(exchange(action)).toJson(QJsonDocument::Compact));
        if (action["action"] == "settings") {
            auto settings = state["settings"].toObject();
            const auto values = action["values"].toObject();
            for (auto i = values.begin(); i != values.end(); ++i) settings[i.key()] = i.value();
            state["settings"] = settings;
            emit snapshotChanged();
        }
        return "{}";
    }
    void theme(const QString &mode) { dispatch(QString::fromUtf8(QJsonDocument(QJsonObject{{"action", "settings"}, {"values", QJsonObject{{"theme", mode}}}}).toJson())); }
    void publish() { emit snapshotChanged(); }
signals:
    void snapshotChanged();
};

static void check(bool condition, const QString &message) {
    if (!condition) throw std::runtime_error(message.toStdString());
}
static QObject *named(QObject *root, const char *name) {
    auto *object = root->findChild<QObject *>(QString::fromLatin1(name));
    check(object != nullptr, QStringLiteral("Missing UI object: ") + name);
    return object;
}
static void invoke(QObject *object, const char *method) {
    check(QMetaObject::invokeMethod(object, method), QStringLiteral("Cannot invoke ") + method);
    QTest::qWait(150);
}
static void click(QQuickWindow *window, QObject *object) {
    auto *item = qobject_cast<QQuickItem *>(object);
    check(item && item->isEnabled(), "Click target unavailable");
    for (auto *ancestor = item->parentItem(); ancestor; ancestor = ancestor->parentItem()) {
        if (ancestor->metaObject()->indexOfProperty("contentY") < 0) continue;
        const qreal top = item->mapToItem(ancestor, QPointF(0, 0)).y();
        const qreal bottom = top + item->height();
        if (top < 0 || bottom > ancestor->height()) {
            ancestor->setProperty("contentY", ancestor->property("contentY").toReal() + (top < 0 ? top : bottom - ancestor->height()));
            QTest::qWait(100);
        }
    }
    const auto point = item->mapToScene(QPointF(item->width()/2, item->height()/2)).toPoint();
    QTest::mouseMove(window, point);
    QTest::qWait(20);
    check(item->isVisible(), "Click target is hidden");
    QTest::mouseClick(window, Qt::LeftButton, Qt::NoModifier, point);
    QTest::qWait(150);
}
static double luminance(QColor color) {
    auto linear = [](double v) { return v <= .04045 ? v / 12.92 : std::pow((v + .055)/1.055, 2.4); };
    return .2126 * linear(color.redF()) + .7152 * linear(color.greenF()) + .0722 * linear(color.blueF());
}

int main(int argc, char **argv) {
    qInfo("Appearance startup: entering Qt initialization");
    QCoreApplication::setAttribute(Qt::AA_ShareOpenGLContexts);
#ifdef Q_OS_MACOS
    // This fixture targets QML menus; the macOS system menu is a separate native smoke check.
    QCoreApplication::setAttribute(Qt::AA_DontUseNativeMenuBar);
#endif
    QtWebEngineQuick::initialize();
    QApplication application(argc, argv);
    qInfo("Appearance startup: application initialized");
    try {
        if (argc > 6 && !QString::fromLocal8Bit(argv[6]).isEmpty())
            check(QResource::registerResource(QString::fromLocal8Bit(argv[6])), "Reference appearance pack failed to load");
        QQuickStyle::setStyle("Basic");
        QTranslator translator, qtTranslator, declarativeTranslator;
        if (argc > 3 && QString::fromLocal8Bit(argv[3]) == "ru") {
            check(translator.load(QFileInfo(QString::fromLocal8Bit(argv[1])).dir().filePath("translations/videodock_ru.qm")), "Russian translations missing");
            const auto translations = QLibraryInfo::path(QLibraryInfo::TranslationsPath);
            if (!qtTranslator.load(QStringLiteral("qt_ru"), translations)) {
                check(qtTranslator.load(QStringLiteral("qtbase_ru"), translations), "Qt base translations missing");
                check(declarativeTranslator.load(QStringLiteral("qtdeclarative_ru"), translations), "Qt Quick translations missing");
                application.installTranslator(&declarativeTranslator);
            }
            application.installTranslator(&qtTranslator);
            application.installTranslator(&translator);
        }
        qmlRegisterType<FixtureBackend>("VideoDock", 1, 0, "Backend");
        QQmlApplicationEngine engine;
        QStringList warnings;
        QObject::connect(&engine, &QQmlEngine::warnings, [&warnings](const QList<QQmlError> &errors) {
            for (const auto &error : errors) warnings << error.toString();
        });
        check(argc >= 3, "Expected Main.qml and screenshot directory");
        engine.load(QUrl::fromLocalFile(QString::fromLocal8Bit(argv[1])));
        qInfo("Appearance startup: QML load returned");
        check(!engine.rootObjects().isEmpty(), "QML failed to load");
        auto *window = qobject_cast<QQuickWindow *>(engine.rootObjects().first());
        check(window != nullptr, "Main is not a window");
        auto *backend = window->findChild<FixtureBackend *>();
        check(backend != nullptr, "No fixture backend");
        engine.globalObject().setProperty("ui", engine.newQObject(window));
        auto evaluate = [&engine](const QString &expression) {
            auto result = engine.evaluate(expression);
            check(!result.isError(), result.toString());
            return result;
        };
        auto capture = [&](const QString &name) {
            qInfo("Appearance checkpoint: %s", qPrintable(name));
            QTest::qWait(150);
            auto image = window->grabWindow();
            check(!image.isNull(), "Empty render");
            check(image.save(QDir(QString::fromLocal8Bit(argv[2])).filePath(name + ".png")), "Screenshot not saved");
        };
        auto paletteColor = [&](QObject *object, const char *role) {
            return engine.newQObject(object).property("palette").property(QString::fromLatin1(role)).toVariant().value<QColor>();
        };
        backend->publish();
        QTest::qWait(350);
        check(window->findChild<QObject *>("mediaToolsDialog") == nullptr, "Removed Media Tools dialog is still instantiated");
        for (auto *object : window->findChildren<QObject *>()) {
            const auto key = object->property("settingKey").toString();
            check(key != "aiProcessingAllowed" && key != "processingNotificationsEnabled", "Removed processing setting is still exposed");
            check(object->property("text").toString() != QCoreApplication::translate("Main", "Processed"), "Removed processed-media tab is still exposed");
        }
        check(named(window, "mainToolbar")->property("height").toInt() == 64, "Toolbar must be 64px");
        check(named(window, "mediaListHeader")->property("height").toInt() == 64, "List header must be 64px");
        check(window->color() == QColor("#292929"), "Default theme must be dark");
        capture("dark-empty");
        auto *homeTitle = qobject_cast<QQuickItem *>(named(window, "browserHomeTitle"));
        auto *homeBrowser = qobject_cast<QQuickItem *>(named(window, "browserPane"));
        check(homeTitle->property("spacing").toInt() == 0, "Browser home title adds a gap absent from the reference");
        check(std::abs(homeTitle->y() - (homeTitle->parentItem()->height() - homeTitle->height() - homeBrowser->height()) / 2) <= 1,
              "Browser home title is not centered between the toolbar and browser as in the reference");
        auto *browserAddress = named(window, "browserAddress");
        check(browserAddress->property("leftPadding").toInt() == 38, "Browser search input does not reserve room for its reference icon");
        check(named(window, "browserSearchIcon")->property("visible").toBool(), "Browser search icon is missing");

        auto *appearance = named(window, "appearanceMenu");
        auto checkSwitchThumb = [&]() {
            auto *control = qobject_cast<QQuickItem *>(named(window, "smartModeSwitch"));
            check(control != nullptr, "Smart Mode control is not a visual item");
            auto *indicator = qvariant_cast<QQuickItem *>(control->property("indicator"));
            check(indicator && !indicator->childItems().isEmpty(), "Smart Mode indicator missing");
            auto *thumbItem = indicator->childItems().first();
            for (const bool enabled : {false, true}) {
                auto settings = backend->state["settings"].toObject();
                settings["smartMode"] = enabled;
                backend->state["settings"] = settings;
                backend->publish();
                const int left = enabled ? 18 : 2;
                QElapsedTimer animation;
                animation.start();
                while (std::abs(thumbItem->x() - left) > 0.5 && animation.elapsed() < 2000) QTest::qWait(20);
                check(std::abs(thumbItem->x() - left) <= 0.5, "Smart Mode thumb did not settle after toggling");
                QTest::qWait(50);
                const QImage frame = window->grabWindow();
                check(!frame.isNull(), "Smart Mode frame unavailable");
                int whitePixels = 0;
                // Sample the painted thumb area rather than one antialiased edge pixel.
                for (int y = 0; y < 20; ++y) for (int x = 0; x < 20; ++x) {
                    const auto pixel = (indicator->mapToScene(QPointF(left + x + 0.5, 2 + y + 0.5)) * window->devicePixelRatio()).toPoint();
                    const auto color = frame.pixelColor(pixel);
                    if (color.red() > 245 && color.green() > 245 && color.blue() > 245) ++whitePixels;
                }
                if (whitePixels < 100) frame.save(QDir(QString::fromLocal8Bit(argv[2])).filePath(enabled ? "smart-mode-on-failure.png" : "smart-mode-off-failure.png"));
                check(whitePixels >= 100, QString("Smart Mode thumb is invisible: enabled=%1 whitePixels=%2").arg(enabled).arg(whitePixels));
            }
        };
        checkSwitchThumb();
        auto openAppearance = [&]() {
            auto *view = engine.newQObject(window).property("menuBar").property("itemAt").call({2}).toQObject();
            click(window, view);
            auto viewMenu = engine.newQObject(named(window, "viewMenu"));
            auto *entry = viewMenu.property("itemAt").call({viewMenu.property("count").toInt() - 1}).toQObject();
            click(window, entry);
            check(appearance->property("visible").toBool(), "View did not open the appearance submenu");
        };
        for (const QString &mode : {"light", "system", "dark"}) {
            openAppearance();
            const auto name = "theme-" + mode.toUtf8();
            click(window, named(window, name.constData()));
            check(backend->state["settings"].toObject()["theme"] == mode, "Appearance menu did not save the selected theme");
            check(named(window, name.constData())->property("checked").toBool(), "Appearance menu lost its selected checkmark");
            checkSwitchThumb();
        }

        auto *intro = named(window, "onboardingDialog");
        invoke(intro, "open");
        check(intro->property("width").toInt() == window->width(), "Onboarding must fill the app instead of opening a small dialog");
        check(intro->property("step").toInt() == 0, "Onboarding must start on the first step");
        auto *introPrimary = qobject_cast<QQuickItem *>(named(window, "onboardingPrimary"));
        const auto primaryPixel = (introPrimary->mapToScene(QPointF(8, introPrimary->height()/2)) * window->devicePixelRatio()).toPoint();
        check(window->grabWindow().pixelColor(primaryPixel) == paletteColor(window, "highlight"), "Onboarding primary action lost its accent background");
        click(window, named(window, "onboardingSecondary"));
        check(!intro->property("visible").toBool() && backend->lastAction["command"] == "onboarding_complete", "Skip did not persist completion");
        invoke(intro, "open");
        click(window, named(window, "onboardingPrimary"));
        check(intro->property("step").toInt() == 1, "Next did not advance onboarding");
        click(window, named(window, "onboardingSecondary"));
        check(intro->property("step").toInt() == 0 && intro->property("visible").toBool(), "Back closed onboarding instead of returning");
        check(engine.newQObject(intro).property("steps").property("length").toInt() == 4, "Onboarding retained a removed Media Tools step");
        for (int step = 0; step < 4; ++step) {
            check(intro->property("step").toInt() == step, "Onboarding skipped a step");
            capture("onboarding-step-" + QString::number(step));
            click(window, named(window, "onboardingPrimary"));
        }
        check(!intro->property("visible").toBool() && backend->lastAction["command"] == "onboarding_complete", "Finish did not persist completion");

        auto *mediaType = named(window, "smartMediaType");
        const bool russian = argc > 3 && QString::fromLocal8Bit(argv[3]) == "ru";
        check(mediaType->property("height").toInt() <= 24, "Smart Mode must use one-line title/value controls, not stacked labels");
        check(mediaType->property("displayText").toString() == (russian ? QString::fromUtf8("Видео") : QStringLiteral("Video")), "Smart Mode exposes an internal media type instead of its translated label");
        mediaType->setProperty("currentIndex", 1);
        check(QMetaObject::invokeMethod(mediaType, "activated", Q_ARG(int, 1)), "Media type activation missing");
        check(backend->lastAction["values"].toObject()["mediaType"] == "audio", "Translated label leaked into the settings protocol");
        mediaType->setProperty("currentIndex", 0);
        check(QMetaObject::invokeMethod(mediaType, "activated", Q_ARG(int, 0)), "Media type activation missing");
        click(window, mediaType);
        auto *mediaMenu = named(window, "smartMediaTypeMenu");
        check(mediaMenu->property("visible").toBool(), "Pointer click did not open the media menu");
        auto *languageEntry = engine.newQObject(mediaMenu).property("itemAt").call({3}).toQObject();
        check(languageEntry != nullptr, "Subtitles submenu entry missing");
        click(window, languageEntry);
        auto *languageMenu = engine.newQObject(languageEntry).property("subMenu").toQObject();
        check(languageMenu && languageMenu->property("visible").toBool(), "Pointer click did not open Subtitles");
        check(languageMenu->property("count").toInt() == 73, "Observed subtitle catalog or separators differ from the reference");
        check(languageMenu->property("width").toReal() >= 200 && languageMenu->property("height").toReal() <= window->height(), "Language popup is zero-width or outside vertical bounds");
        QObject *russianEntry = nullptr;
        int russianIndex = -1;
        auto languageApi = engine.newQObject(languageMenu);
        for (int index = 0; index < languageMenu->property("count").toInt(); ++index) {
            auto *entry = languageApi.property("itemAt").call({index}).toQObject();
            if (entry && entry->property("modelData").toString() == "ru") {
                russianEntry = entry;
                russianIndex = index;
            }
        }
        check(russianEntry != nullptr, "Russian subtitle option missing from observed catalog");
        // Menu items outside ListView's realized range have no usable click geometry yet.
        languageApi.property("contentItem").property("positionViewAtIndex").call({russianIndex, 1});
        QTest::qWait(150);
        click(window, russianEntry);
        check(backend->lastAction["values"].toObject()["subtitles"].toArray().contains("ru"), "Pointer language toggle did not dispatch the canonical ID");
        check(languageMenu->property("visible").toBool() && mediaMenu->property("visible").toBool(), "Language toggle closed the multi-selection menus");
        invoke(mediaMenu, "close");
        invoke(named(window, "manualDownloadDialog"), "open");
        named(window, "manualContainer")->setProperty("currentIndex", 0);
        check(named(window, "manualContainer")->property("displayText").toString() == (russian ? QString::fromUtf8("Любой") : QStringLiteral("Any")), "Manual filters expose untranslated protocol values");
        invoke(named(window, "manualDownloadDialog"), "close");

        // Every menu label must fit, including translated labels and shortcuts.
        QObject *fileMenu = nullptr;
        for (auto *object : window->findChildren<QObject *>())
            if (object->property("title").toString() == (russian ? QString::fromUtf8("Файл") : QStringLiteral("File")) && object->metaObject()->indexOfProperty("count") >= 0)
                fileMenu = object;
        check(fileMenu, "File menu missing");
        invoke(fileMenu, "open");
        auto fileMenuApi = engine.newQObject(fileMenu);
        for (int index = 0; index < fileMenu->property("count").toInt(); ++index) {
            auto *entry = fileMenuApi.property("itemAt").call({index}).toQObject();
            auto *content = entry ? qvariant_cast<QQuickItem *>(entry->property("contentItem")) : nullptr;
            if (content && !entry->property("text").toString().isEmpty())
                check(content->width() + 1 >= content->implicitWidth(), "File menu truncates a label");
        }
        invoke(fileMenu, "close");

        QApplication::clipboard()->setText("private-fixture-marker");
        backend->lastAction = {};
        click(window, named(window, "pasteButton"));
        check(named(window, "urlDialog")->property("visible").toBool(), "Non-URL clipboard did not open manual link entry");
        check(backend->lastAction["action"] != "parse", "Non-URL clipboard was dispatched to the engine");
        check(named(window, "clipboardBuffer")->property("text").toString().isEmpty(), "Hidden clipboard buffer retained private input");
        invoke(named(window, "urlDialog"), "close");

        QObject *countLabel = nullptr;
        for (auto *object : window->findChildren<QObject *>())
            if (object->property("text").toString() == (russian ? QString::fromUtf8("0 элементов") : QStringLiteral("0 items")))
                countLabel = object;
        check(countLabel, "Item count label missing");
        for (const auto &expected : {qMakePair(1, QString::fromUtf8("1 элемент")), qMakePair(2, QString::fromUtf8("2 элемента")), qMakePair(5, QString::fromUtf8("5 элементов")), qMakePair(21, QString::fromUtf8("21 элемент"))}) {
            QJsonArray counted;
            for (int index = 0; index < expected.first; ++index)
                counted.append(QJsonObject{{"id", QString::number(index)}, {"kind", "video"}});
            backend->state["jobs"] = counted;
            backend->publish();
            QTest::qWait(100);
            const auto english = QString::number(expected.first) + (expected.first == 1 ? " item" : " items");
            check(countLabel->property("text").toString() == (russian ? expected.second : english), "Item count uses the wrong plural form");
        }
        backend->state["jobs"] = QJsonArray{};
        backend->publish();

        // Paste reads the real Qt clipboard through the shipped Main.qml.
        {
            const auto saved = backend->state["jobs"];
            backend->state["jobs"] = QJsonArray{
                QJsonObject{{"id", "old"}, {"created_at", 100}, {"kind", "video"}},
                QJsonObject{{"id", "recent"}, {"created_at", 101}, {"kind", "video"}},
                QJsonObject{{"id", "newest"}, {"created_at", 101}, {"kind", "video"}},
                QJsonObject{{"id", "zz-library"}, {"created_at", 0}, {"kind", "video"}, {"_library", true}}
            };
            backend->publish();
            QTest::qWait(100);
            check(evaluate("ui.listRows().map(function(j){return j.id}).join(',')").toString() == "newest,recent,old,zz-library", "New downloads are not above older jobs and imported library rows");
            QJsonArray many;
            for (int index = 0; index < 50; ++index)
                many.append(QJsonObject{{"id", QString("older-%1").arg(index)}, {"kind", "video"}, {"created_at", 100}});
            backend->state["jobs"] = many;
            backend->publish();
            QTest::qWait(100);
            auto *orderingList = qobject_cast<QQuickItem *>(named(window, "downloadsList"));
            orderingList->setProperty("contentY", 384);
            QTest::qWait(50);
            check(orderingList->property("contentY").toReal() > 300, "Ordering scroll fixture did not scroll");
            many.append(QJsonObject{{"id", "just-pasted"}, {"kind", "video"}, {"created_at", 100}});
            backend->state["jobs"] = many;
            backend->publish();
            QTest::qWait(150);
            check(evaluate("ui.listRows()[0].id").toString() == "just-pasted", "Same-second insertion did not become the first row");
            check(orderingList->property("contentY").toReal() <= orderingList->property("originY").toReal() + 1, "New top row is outside the visible viewport");
            backend->state["jobs"] = saved;
            backend->publish();
        }
        window->requestActivate();
        QTest::qWait(100);
        for (const QString &url : {"https://youtu.be/fixture", "https://www.tiktok.com/@fixture/video/1", "https://vimeo.com/1"}) {
            evaluate("ui.sortKey='name'; ui.descending=false; ui.activeTab='Audio'; ui.search='previous filter'");
            QApplication::clipboard()->setText(url);
            backend->lastAction = {};
            click(window, named(window, "pasteButton"));
            check(backend->lastAction["action"] == "parse" && backend->lastAction["url"] == url, "Paste Link lost the clipboard source");
            check(evaluate("ui.sortKey==='date' && ui.descending && ui.activeTab==='All' && ui.search===''").toBool(), "Pasting did not reveal the newest download at the top");
            backend->lastAction = {};
            QTest::keySequence(window, QKeySequence(QKeySequence::Paste));
            QTest::qWait(150);
            check(backend->lastAction["action"] == "parse" && backend->lastAction["url"] == url, "Ctrl+V lost the clipboard source");
        }

        backend->state["jobs"] = QJsonArray{
            QJsonObject{{"id", "fixture-video"}, {"created_at", 3}, {"title", "A very long video title to check clipping without hiding the controls"}, {"url", "https://example.invalid/video"}, {"kind", "video"}, {"state", "downloading"}, {"progress", 42}, {"format", "MP4"}, {"quality", "1080p"}, {"speed", "2.4 MB/s"}},
            QJsonObject{{"id", "fixture-audio"}, {"created_at", 1}, {"title", "Audio fixture"}, {"kind", "audio"}, {"state", "complete"}, {"format", "MP3"}, {"quality", "192 kbps"}},
            QJsonObject{{"id", "fixture-error"}, {"created_at", 2}, {"title", "Unavailable media"}, {"kind", "video"}, {"state", "error"}, {"error", "The service is unavailable. Retry later."}}
        };
        backend->publish();
        QTest::qWait(250);
        // ListView delegates are visual children, not necessarily QObject descendants of the window.
        auto *list = qobject_cast<QQuickItem *>(named(window, "downloadsList"));
        QList<QQuickItem *> rows;
        std::function<void(QQuickItem *)> checkRows = [&](QQuickItem *parent) {
            for (auto *item : parent->childItems()) {
                if (item->objectName() == "downloadRow") {
                    check(item->height() == 64, "Row must be 64px");
                    rows.append(item);
                }
                checkRows(item);
            }
        };
        checkRows(list);
        check(rows.size() == 3, "Expected three visible download rows");
        const QSize originalSize = window->size();
        for (int width : {1000, 2200, 1280, 2560, 1100}) {
            window->resize(width, originalSize.height());
            auto job = backend->state["jobs"].toArray().first().toObject();
            auto jobs = backend->state["jobs"].toArray();
            job["progress"] = width % 100;
            jobs.replace(0, job);
            backend->state["jobs"] = jobs;
            backend->publish();
            QTest::qWait(80);
            rows.clear(); checkRows(list);
            check(rows.size() == 3, "Resize or progress update lost download rows");
            for (auto *item : rows) {
                const qreal left = item->mapToItem(list, QPointF()).x();
                const qreal expectedLeft = (list->width() - item->width()) / 2;
                if (std::abs(left - expectedLeft) > 1)
                    qWarning() << "Download alignment:" << list->width() << left << expectedLeft;
                check(std::abs(left - expectedLeft) <= 1, "Download cards diverged after resize or progress update");
                check(left >= 0 && left + item->width() <= list->width() + 1, "Download card escaped the viewport");
            }
        }
        window->resize(originalSize);
        QTest::qWait(80);
        const QList<QPair<QString, QString>> platformIcons{
            {"https://youtu.be/fixture", "service-youtube.svg"},
            {"https://WWW.YOUTUBE.COM/watch?v=fixture", "service-youtube.svg"},
            {"https://vm.tiktok.com/fixture", "service-tiktok.svg"},
            {"https://vimeo.com/1", "service-vimeo.svg"},
            {"https://soundcloud.com/fixture/clip", "service-soundcloud.svg"},
            {"https://fb.watch/fixture", "service-facebook.svg"},
            {"https://twitch.tv/fixture", "service-twitch.svg"},
            {"https://dai.ly/fixture", "service-dailymotion.svg"},
            {"https://b23.tv/fixture", "service-bilibili.svg"},
            {"https://rumble.com/fixture", "service-rumble.svg"},
            {"https://m.tv.naver.com/1", "service-naver.svg"},
            {"https://fixture.tumblr.com/post/1", "service-tumblr.svg"},
            {"https://nico.ms/fixture", "service-nicovideo.svg"},
            {"https://likee.video/fixture", "service-likee.svg"},
            {"https://flic.kr/fixture", "service-flickr.svg"},
            {"https://www.bitchute.com/video/fixture", "service-bitchute.svg"},
            {"https://youtube.com.example.invalid/fixture", "browser.svg"}
        };
        const QJsonValue originalJobs = backend->state["jobs"];
        auto platformJob = originalJobs.toArray().first().toObject();
        for (const auto &fixture : platformIcons) {
            platformJob["url"] = fixture.first;
            backend->state["jobs"] = QJsonArray{platformJob};
            backend->publish();
            QTest::qWait(150);
            rows.clear(); checkRows(list);
            check(rows.size() == 1, "Platform fixture row missing");
            auto *icon = named(rows.first(), "downloadPlatformIcon");
            check(icon->property("visible").toBool() && icon->property("status").toInt() == 1, "Platform icon did not render");
            const QString source = icon->property("source").toUrl().toString();
            const QString reference = fixture.second.mid(QStringLiteral("service-").size()).replace("tiktok", "tik-tok").replace(".svg", "-icon.svg");
            check(source.endsWith(fixture.second) || source.endsWith(reference), "Wrong platform icon for source host");
        }
        backend->state["jobs"] = originalJobs;
        backend->publish();
        QTest::qWait(150);
        rows.clear(); checkRows(list);
        check(rows.size() == 3, "Platform tests did not restore the download fixtures");

        click(window, rows.first());
        check(evaluate("ui.selectedIds.length").toInt() == 1, "Row click did not select a download");
        QTest::mouseClick(window, Qt::LeftButton, Qt::ControlModifier, rows[1]->mapToScene(QPointF(rows[1]->width()/2, rows[1]->height()/2)).toPoint());
        QTest::qWait(100);
        check(evaluate("ui.selectedIds.length").toInt() == 2, "Ctrl+click did not extend selection");
        evaluate("ui.selectedIds = ['fixture-video']");
        capture("dark-downloads");

        auto checkContextMenu = [&](bool subscriptions) {
            rows.clear(); checkRows(list);
            check(!rows.isEmpty(), "Context menu has no fixture row");
            auto *row = rows.first();
            QTest::mouseClick(window, Qt::RightButton, Qt::NoModifier, row->mapToScene(QPointF(row->width()/2, row->height()/2)).toPoint());
            QTest::qWait(150);
            auto menu = engine.newQObject(named(window, "downloadContextMenu"));
            check(menu.property("visible").toBool(), "Context menu did not open");
            QQuickItem *previous = nullptr;
            bool previousSeparator = false;
            int subscriptionActions = 0;
            for (int i = 0; i < menu.property("count").toInt(); ++i) {
                auto *item = qobject_cast<QQuickItem *>(menu.property("itemAt").call({i}).toQObject());
                check(item != nullptr, "Context menu item is missing");
                const auto text = item->property("text").toString();
                if (text == QCoreApplication::translate("JobList", "Edit Subscription Settings") || text == QCoreApplication::translate("JobList", "Unsubscribe")) {
                    ++subscriptionActions;
                    check(item->isVisible() == subscriptions, "Subscription context action has wrong visibility");
                }
                if (!item->isVisible()) {
                    check(item->height() == 0, "Hidden context action reserves an empty menu row");
                    continue;
                }
                const bool separator = item->inherits("QQuickMenuSeparator");
                if (separator) {
                    check(item->height() <= 9, "Context separator adds excessive vertical space");
                } else {
                    check(item->property("leftPadding").toReal() >= 12 && item->property("rightPadding").toReal() >= 12, "Context action lacks horizontal padding");
                    auto *content = qvariant_cast<QQuickItem *>(item->property("contentItem"));
                    check(content && content->implicitWidth() <= content->width() + 1, "Context action text is truncated");
                    check(item->height() <= 34, "Context action has excessive vertical padding");
                }
                check(!separator || (previous && !previousSeparator), "Context menu has a leading or duplicate separator");
                if (previous) {
                    const qreal gap = item->mapToScene(QPointF()).y() - previous->mapToScene(QPointF(0, previous->height())).y();
                    check(std::abs(gap) <= 1, "Context menu contains a gap between visible actions");
                }
                previous = item;
                previousSeparator = separator;
            }
            check(subscriptionActions == 2 && previous && previous->objectName() == "deleteAction", "Delete must follow the last visible context section");
            capture(subscriptions ? "context-subscriptions" : "context-downloads");
            invoke(named(window, "downloadContextMenu"), "close");
        };
        checkContextMenu(false);
        {
            auto *row = rows.first();
            QQuickItem *more = nullptr;
            for (auto *item : row->findChildren<QQuickItem *>()) {
                if (item->property("text").toString() == QCoreApplication::translate("JobList", "More actions")) more = item;
            }
            check(more != nullptr, "More actions button is missing");
            const QPointF button = more->mapToScene(QPointF(more->width()/2, more->height()/2));
            QCursor::setPos(window->mapToGlobal(QPoint(10, window->height()-10)));
            QTest::mouseClick(window, Qt::LeftButton, Qt::NoModifier, button.toPoint());
            QTest::qWait(150);
            auto *menu = named(window, "downloadContextMenu");
            auto *content = qvariant_cast<QQuickItem *>(menu->property("contentItem"));
            check(menu->property("visible").toBool() && content, "More actions did not open a menu");
            const QPointF position = content->mapToScene(QPointF());
            check(std::abs(position.x()-button.x()) <= content->width()+more->width() && std::abs(position.y()-button.y()) <= content->height()+more->height(), "More actions menu opened away from its button");
            capture("context-button-downloads");
            invoke(menu, "close");
        }
        backend->state["subscriptions"] = QJsonArray{QJsonObject{{"id", "fixture-subscription"}, {"title", "Subscription fixture"}, {"kind", "channel"}, {"state", "subscribed"}}};
        backend->publish();
        evaluate("ui.activeTab = 'Subscriptions'");
        QTest::qWait(150);
        checkContextMenu(true);
        evaluate("ui.activeTab = 'All'; ui.selectedIds = []; ui.focusedJob = null");
        backend->state["subscriptions"] = QJsonArray{};
        backend->publish();
        QTest::qWait(150);

        auto *settings = named(window, "settingsDialog");
        click(window, named(window, "preferencesButton"));
        check(!named(window, "autoStart")->property("enabled").toBool(), "Unsupported autostart is offered in settings");
        settings->setProperty("tab", 1);
        QTest::qWait(150);
        QList<QQuickItem *> pending{qvariant_cast<QQuickItem *>(settings->property("contentItem"))};
        QQuickItem *sleepToggle = nullptr;
        QList<QQuickItem *> intensityLabels(5, nullptr);
        while (!pending.isEmpty()) {
            auto *item = pending.takeLast();
            check(item != nullptr, "Settings content is missing");
            if (item->objectName() == "preventSleepEnabled") sleepToggle = item;
            for (int index = 0; index < intensityLabels.size(); ++index)
                if (item->objectName() == "simultaneousLabel" + QString::number(index)) intensityLabels[index] = item;
            pending.append(item->childItems());
        }
        check(sleepToggle && !sleepToggle->isEnabled(), "Unsupported sleep inhibition is offered in settings");
        for (int index = 1; index < 5; ++index) {
            auto *previous = intensityLabels[index - 1];
            auto *current = intensityLabels[index];
            check(previous && current, "Simultaneous download label is missing");
            check(previous->x() + previous->width() + 6 <= current->x(), "Simultaneous download labels overlap or lack spacing");
        }
        settings->setProperty("tab", 0);
        QTest::qWait(150);
        check(settings->property("visible").toBool(), "Preferences button did not open dialog");
        check(settings->property("width").toInt() == 800 + settings->property("sidebarExtra").toInt(), "Settings width must include measured sidebar localization allowance");
        for (const QString &mode : {"dark", "light", "system"}) {
            backend->theme(mode);
            QTest::qWait(100);
            check(backend->state["settings"].toObject()["theme"] == mode, "Theme was not dispatched");
            for (int tab = 0; tab < 5; ++tab) {
                settings->setProperty("tab", tab);
                capture(mode + "-settings-" + QString::number(tab));
            }
            const QColor fg = paletteColor(window, "text"), bg = paletteColor(window, "window");
            check((std::max(luminance(fg), luminance(bg)) + .05)/(std::min(luminance(fg), luminance(bg)) + .05) >= 4.5, "Text contrast below 4.5:1");
        }
        click(window, named(window, "settingsCloseButton"));
        check(!settings->property("visible").toBool(), "Top-right Close did not close settings");
        for (const QString &mode : {"dark", "light"}) {
            backend->theme(mode);
            for (int width : {1150, 950}) {
                window->setWidth(width);
                QTest::qWait(100);
                auto *paste = qobject_cast<QQuickItem *>(named(window, "pasteButton"));
                auto *panel = qobject_cast<QQuickItem *>(named(window, "controlPanelOpenerButton"));
                capture("toolbar-geometry-" + mode + "-" + QString::number(width));
                check(paste->mapToScene(QPointF(0, 0)).x() >= 0 && panel->mapToScene(QPointF(panel->width(), 0)).x() <= width, QString("Toolbar clipped: width=%1 panelRight=%2").arg(width).arg(panel->mapToScene(QPointF(panel->width(), 0)).x()));
                capture(mode + "-downloads-" + QString::number(width));
            }
            window->setWidth(1150);
            QTest::qWait(100);
            click(window, named(window, "sortButton"));
            check(named(window, "sortMenu")->property("visible").toBool(), "Sort menu unavailable");
            capture(mode + "-sort");
            invoke(named(window, "sortMenu"), "close");
            for (const char *name : {"manualDownloadDialog", "transferDialog", "detailsDialog", "onboardingDialog", "urlDialog", "removeDialog", "controlPanel", "about", "quitDialog"}) {
                auto *dialog = named(window, name);
                if (QString::fromLatin1(name) == "manualDownloadDialog") {
                    backend->state["manual"] = QJsonObject{{"title", "Format selection fixture"}, {"state", "ready"}, {"kind", "video"}, {"url", "https://example.invalid/video"}, {"formats", QJsonArray{
                        QJsonObject{{"id", "mp4"}, {"media_type", "video"}, {"quality", "1080p"}, {"container", "mp4"}},
                        QJsonObject{{"id", "mp4-720"}, {"media_type", "video"}, {"quality", "720p"}, {"container", "mp4"}}
                    }}};
                    backend->publish();
                }
                invoke(dialog, "open");
                check(dialog->property("visible").toBool(), QString::fromLatin1(name) + " is hidden");
                check(paletteColor(dialog, "text") == paletteColor(window, "text"), QString::fromLatin1(name) + " lost theme text");
                capture(mode + "-" + QString::fromLatin1(name));
                if (QString::fromLatin1(name) == "manualDownloadDialog") {
                    check(named(window, "manualHeader")->property("height").toInt() == 128, "Manual header must be 128px");
                    check(named(window, "manualFooter")->property("height").toInt() == 88, "Manual footer must be 88px");
                    auto *container = named(window, "manualContainer");
                    container->setProperty("currentIndex", 1);
                    QTest::qWait(100);
                    check(engine.newQObject(dialog).property("formats").property("length").toInt() == 2, "Selecting MP4 hides its matching formats");
                    container->setProperty("currentIndex", 0);
                    auto *options = qobject_cast<QQuickItem *>(named(window, "formatOptions"));
                    QQuickItem *chosen = nullptr;
                    std::function<void(QQuickItem *)> findOption = [&](QQuickItem *parent) {
                        for (auto *item : parent->childItems()) {
                            if (item->objectName() == "formatOption" && item->property("text").toString().contains("720p")) chosen = item;
                            findOption(item);
                        }
                    };
                    findOption(options);
                    check(chosen != nullptr, "720p format option missing");
                    click(window, chosen);
                    check(dialog->property("selectedFormatId").toString() == "mp4-720", "Radio click did not select 720p");
                    click(window, named(window, "downloadButton"));
                    check(backend->lastAction["filename"].toString().isEmpty(), "Opening manual download invented a filename before editing");
                    check(backend->lastAction["format"].toObject()["media_type"] == "video" && backend->lastAction["format"].toObject()["container"] == "Any", "Translated manual filter leaked into the download protocol");
                    check(backend->lastAction["action"] == "download" && backend->lastAction["format_id"] == "mp4-720", QStringLiteral("Selected format did not reach download action: ") + QString::fromUtf8(QJsonDocument(backend->lastAction).toJson(QJsonDocument::Compact)));
                    invoke(dialog, "open");
                    auto *manualPath = named(window, "manualPath");
                    manualPath->setProperty("text", "/tmp/out/renamed.mp4");
                    manualPath->setProperty("edited", true);
                    click(window, named(window, "downloadButton"));
                    check(backend->lastAction["output_dir"] == "/tmp/out" && backend->lastAction["filename"] == "renamed.mp4", "Edited file path did not preserve separate output directory and filename");
#ifdef Q_OS_WIN
                    const char *windowsPaths[][2] = {
                        {R"(C:\Videos\renamed.mp4)", "C:/Videos"},
                        {"C:/renamed.mp4", "C:/"},
                        {R"(C:\renamed.mp4)", "C:/"},
                        {R"(\\server\share\renamed.mp4)", "//server/share"},
                        {R"(C:\Videos/sub\renamed.mp4)", "C:/Videos/sub"}
                    };
                    for (const auto &path : windowsPaths) {
                        invoke(dialog, "open");
                        manualPath->setProperty("text", QString::fromLatin1(path[0]));
                        manualPath->setProperty("edited", true);
                        click(window, named(window, "downloadButton"));
                        check(backend->lastAction["output_dir"].toString() == QString::fromLatin1(path[1]), "Windows output path changed its destination: " + QString::fromLatin1(path[0]));
                        check(backend->lastAction["filename"] == "renamed.mp4", "Windows output path changed its filename");
                    }
#endif
                    invoke(dialog, "open");
                    manualPath->setProperty("text", "invalid-file-name");
                    manualPath->setProperty("edited", true);
                    const auto beforeInvalidPath = backend->lastAction;
                    click(window, named(window, "downloadButton"));
                    check(backend->lastAction == beforeInvalidPath && dialog->property("visible").toBool(), "Invalid file path was dispatched");
                    invoke(dialog, "close");
                    auto manual = backend->state["manual"].toObject();
                    manual["kind"] = "playlist";
                    manual["items"] = QJsonArray{QJsonObject{{"id", "one"}, {"title", "First item"}}, QJsonObject{{"id", "two"}, {"title", "Second item"}}};
                    backend->state["manual"] = manual;
                    backend->publish();
                    invoke(dialog, "open");
                    capture(mode + "-manual-batch");
                    invoke(dialog, "close");
                    const auto savedSettings = backend->state["settings"].toObject();
                    auto historySettings = savedSettings;
                    historySettings["paths"] = QJsonArray{"/fixture/first", "/fixture/second"};
                    historySettings["mediaType"] = "video";
                    historySettings["container"] = "mp4";
                    historySettings["videoVR"] = "Any";
                    backend->state["settings"] = historySettings;
                    backend->publish();
                    invoke(dialog, "open");
                    auto *system = named(window, "manualOperatingSystem");
                    check(system->property("visible").toBool(), "Batch OS presets are missing");
                    auto actionForDialog = [&]() { return QJsonObject::fromVariantMap(engine.newQObject(dialog).property("downloadAction").call().toVariant().toMap()); };
                    auto *type = named(window, "manualMediaType");
                    for (int audio = 0; audio < 2; ++audio) {
                        type->setProperty("currentIndex", audio);
                        check(QMetaObject::invokeMethod(type, "activated", Q_ARG(int, audio)), "Manual media choice missing");
                        const QStringList containers = audio ? QStringList{"MP3", "M4A", "OGG", "M4A", "MP3"} : QStringList{"MP4", "MP4", "MKV", "MP4", "MP4"};
                        for (int alias = 1; alias <= 5; ++alias) {
                            system->setProperty("currentIndex", alias);
                            check(QMetaObject::invokeMethod(system, "activated", Q_ARG(int, alias)), "OS preset activation missing");
                            const auto action = actionForDialog();
                            check(action["format"].toObject()["container"] == containers[alias - 1], "OS preset produced the wrong container");
                            check(action["format_id"] == "" && action["format"].toObject()["video_codec"] == "Any", "OS preset retained a pinned source codec");
                        }
                    }
                    click(window, named(window, "manualOutputFolder"));
                    auto *history = named(window, "manualFolderHistory");
                    check(history->property("visible").toBool(), "Batch folder history did not open");
                    capture(mode + "-manual-folder-history");
                    auto *folderEntry = engine.newQObject(history).property("itemAt").call({1}).toQObject();
                    check(folderEntry && folderEntry->property("text").toString() == "/fixture/second", "Saved folder history is incomplete");
                    invoke(folderEntry, "triggered");
                    invoke(history, "close");
                    check(dialog->property("outputDirectory").toString() == "/fixture/second", "Folder history did not select the output directory");
                    check(backend->state["settings"] == historySettings, "Manual choices modified global preferences");
                    invoke(dialog, "close");
                    const QJsonObject preferences{{"media_type", "audio"}, {"container", "mp3"}, {"quality", 720}, {"video_codec", "h264"}, {"audio_codec", "aac"}, {"audio_bitrate", 192}, {"fps", 30}, {"vr", false}, {"vr_mode", "Any"}, {"subtitles", QJsonArray{"ru", "en:auto"}}, {"audio_tracks", QJsonArray{"ru"}}, {"format_id", QJsonValue::Null}};
                    manual["kind"] = "channel";
                    manual["mode"] = "subscribe";
                    manual["editing"] = true;
                    manual["download_existing"] = true;
                    manual["preferences"] = preferences;
                    manual["output_dir"] = "/fixture/subscription";
                    manual["selected_format_id"] = "audio-192";
                    manual["formats"] = QJsonArray{
                        QJsonObject{{"id", "video"}, {"media_type", "video"}, {"quality", "1080p"}, {"container", "MP4"}},
                        QJsonObject{{"id", "audio-320"}, {"media_type", "audio"}, {"quality", "320 kbps"}, {"audio_codec", "aac"}, {"container", "M4A"}},
                        QJsonObject{{"id", "audio-192"}, {"media_type", "audio"}, {"quality", "192 kbps"}, {"audio_codec", "aac"}, {"container", "M4A"}}
                    };
                    backend->state["manual"] = QJsonObject{{"state", "parsing"}, {"kind", "video"}, {"formats", QJsonArray{}}};
                    backend->publish();
                    invoke(dialog, "open");
                    backend->state["manual"] = manual;
                    backend->publish();
                    QTest::qWait(50);
                    check(type->property("currentText").toString() == "audio" && container->property("currentText").toString() == "MP3", "Subscription edit inherited global format preferences");
                    check(dialog->property("selectedFormatId").toString() == "audio-192", "Subscription edit selected the first format instead of its preference");
                    check(!named(window, "manualDownloadExisting")->property("visible").toBool(), "Subscription edit offers Download Existing");
                    check(named(window, "downloadButton")->property("text").toString() == (russian ? QString::fromUtf8("Сохранить") : QStringLiteral("Save")), "Subscription edit has no Save action");
                    capture(mode + "-subscription-edit");
                    click(window, named(window, "downloadButton"));
                    auto verifyPreserved = [&](const QString &expectedContainer) {
                        const auto action = backend->lastAction;
                        check(action["mode"] == "subscribe", "Asynchronous subscription edit submitted a download instead of Save");
                        const auto format = action["format"].toObject();
                        for (const auto &key : {"media_type", "quality", "video_codec", "audio_codec", "audio_bitrate", "fps", "vr", "vr_mode"})
                            check(format[key] == preferences[key], QString("Subscription preference lost: ") + key);
                        check(format["container"] == expectedContainer && action["subtitles"] == preferences["subtitles"] && action["audio_tracks"] == preferences["audio_tracks"], "Subscription edit changed container or languages unexpectedly");
                        check(action["output_dir"] == "/fixture/subscription" && action["download_existing"] == false && action["format_id"] == "", "Subscription edit changed destination or requested existing downloads");
                    };
                    verifyPreserved("mp3");
                    invoke(dialog, "open");
                    container->setProperty("currentIndex", 3);
                    check(QMetaObject::invokeMethod(container, "activated", Q_ARG(int, 3)), "Container activation missing");
                    click(window, named(window, "downloadButton"));
                    verifyPreserved("OGG");
                    auto vrPreferences = preferences;
                    vrPreferences["media_type"] = "video";
                    vrPreferences["container"] = "mp4";
                    vrPreferences["vr_mode"] = QString::fromUtf8("3D/180°");
                    manual["preferences"] = vrPreferences;
                    manual["selected_format_id"] = "stereo";
                    manual["formats"] = QJsonArray{QJsonObject{{"id", "stereo"}, {"media_type", "video"}, {"quality", "720p"}, {"container", "MP4"}, {"vr", QString::fromUtf8("3D/180°")}}};
                    backend->state["manual"] = manual;
                    backend->publish();
                    invoke(dialog, "open");
                    auto *savedVR = named(window, "manualVR");
                    check(savedVR->property("visible").toBool() && savedVR->property("displayText").toString() == QString::fromUtf8("3D/180°"), "Subscription VR preference is hidden or displays the global value");
                    invoke(dialog, "close");
                    backend->state["settings"] = savedSettings;
                    backend->state.remove("manual");
                    backend->publish();
                } else {
                    invoke(dialog, "close");
                }
            }
            evaluate("ui.browserVisible = false");
            QTest::qWait(100);
            auto *browserPane = named(window, "browserPane");
            check(browserPane->property("height").toInt() == 56, "Browser did not minimize");
            click(window, named(window, "browserAddress"));
            check(evaluate("ui.browserVisible").toBool(), "Browser address did not expand browser");
            capture(mode + "-browser");
            click(window, named(window, "hideBrowserButton"));
            check(!evaluate("ui.browserVisible").toBool(), "Browser close did not minimize browser");
        }
        {
            auto *browser = named(window, "browserPane");
            check(QMetaObject::invokeMethod(browser, "go", Q_ARG(QVariant, QVariant(QStringLiteral("http://127.0.0.1:9/browser-fixture")))), "Browser navigation method missing");
            QTest::qWait(250);
            check(window->findChild<QObject *>("webBrowser") != nullptr, "Navigation did not create the browser page");
            click(window, named(window, "browserHomeButton"));
            QTest::qWait(150);
            check(window->findChild<QObject *>("webBrowser") == nullptr, "Home retained the old browser page and history");
            check(browser->property("home").toBool(), "Home did not return to the service grid");
            QTcpServer blockedOrigin;
            check(blockedOrigin.listen(QHostAddress::LocalHost), "Cannot start proxy bypass sentinel");
            for (bool restart : {false, true}) {
                backend->state["browserProxySupported"] = false;
                backend->state["browserProxyRestartRequired"] = restart;
                backend->publish();
                QTest::qWait(50);
                check(QMetaObject::invokeMethod(browser, "go", Q_ARG(QVariant, QVariant(QStringLiteral("http://127.0.0.1:%1/proxy-bypass").arg(blockedOrigin.serverPort())))), "Blocked navigation method missing");
                QTest::qWait(150);
                check(window->findChild<QObject *>("webBrowser") == nullptr && !blockedOrigin.hasPendingConnections(), "Blocked proxy navigation reached origin directly");
                check(!evaluate("ui.localNotice").toString().isEmpty(), "Blocked proxy has no user-facing explanation");
            }
            backend->state.remove("browserProxySupported");
            backend->state.remove("browserProxyRestartRequired");
            backend->publish();
            evaluate("ui.localNotice = ''; ui.browserVisible = false");
        }
        if (argc > 5) {
            QTemporaryDir profile;
            check(profile.isValid(), "Cannot create isolated Engine profile");
            const auto cleanupEngine = qScopeGuard([&]() {
                if (backend->engineProcess.state() == QProcess::NotRunning) return;
                backend->engineProcess.closeWriteChannel();
                if (!backend->engineProcess.waitForFinished(5000)) {
                    backend->engineProcess.kill();
                    backend->engineProcess.waitForFinished(5000);
                }
            });
            const QString driver = QString::fromLocal8Bit(argv[4]);
            backend->startEngine(driver, profile.path());
            backend->dispatch(QString::fromUtf8(QJsonDocument(QJsonObject{{"action", "settings"}, {"values", QJsonObject{{"path", profile.path() + "/downloads"}, {"smartMode", false}}}}).toJson()));
            check(backend->state["settings"].toObject()["theme"] == "dark", "Real Engine theme default differs");
            openAppearance();
            click(window, named(window, "theme-light"));
            check(backend->state["settings"].toObject()["theme"] == "light", "Real Engine rejected theme choice");
            backend->stopEngine();
            backend->startEngine(driver, profile.path());
            check(backend->state["settings"].toObject()["theme"] == "light" && window->color() == QColor("#f6f6f6"), "Theme did not survive actual Engine restart");

            click(window, named(window, "preferencesButton"));
            settings->setProperty("tab", 2);
            settings->setProperty("proxyEditing", true);
            auto *speed = named(window, "speedLimitChoice");
            speed->setProperty("currentIndex", 5);
            QMetaObject::invokeMethod(speed, "activated", Q_ARG(int, 5));
            QTest::qWait(100);
            check(backend->state["settings"].toObject()["speedLimit"].toInt() == 125000, "1 Mbps was not normalized by real Engine");
            check(speed->property("currentIndex").toInt() == 5, "UI displays Unlimited after saving a 1 Mbps speed limit");
            auto *host = named(window, "proxyHost");
            host->setProperty("text", "invalid host");
            click(window, named(window, "saveProxySettings"));
            check(!evaluate("ui.localNotice").toString().isEmpty(), "Real validation error is not visible");
            check(backend->state["settings"].toObject()["proxyHost"].toString().isEmpty(), "Invalid proxy host was saved");
            host->setProperty("text", "127.0.0.1");
            named(window, "proxyPort")->setProperty("text", "70000");
            check(!named(window, "saveProxySettings")->property("enabled").toBool(), "Invalid proxy port can be submitted");
            named(window, "proxyPort")->setProperty("text", "1080");
            click(window, named(window, "saveProxySettings"));
            check(backend->state["settings"].toObject()["proxyHost"] == "127.0.0.1", "Proxy fields did not reach real Engine");
            capture("real-engine-settings");
            invoke(settings, "close");
            backend->theme("dark");

            QFile sample(QString::fromLocal8Bit(argv[5]));
            check(sample.open(QIODevice::ReadOnly), "Local media sample missing");
            const QByteArray media = sample.readAll();
            QTcpServer server;
            check(server.listen(QHostAddress::LocalHost), "Cannot start local media fixture");
            QObject::connect(&server, &QTcpServer::newConnection, [&]() {
                while (auto *socket = server.nextPendingConnection()) {
                    QObject::connect(socket, &QTcpSocket::readyRead, socket, [socket, media]() {
                        auto request = socket->property("request").toByteArray() + socket->readAll();
                        socket->setProperty("request", request);
                        if (!request.contains("\r\n\r\n")) return;
                        qint64 offset = 0;
                        for (const auto &line : request.split('\n')) {
                            if (line.toLower().startsWith("range: bytes=")) offset = line.mid(13).split('-').first().toLongLong();
                        }
                        offset = qBound(qint64(0), offset, qint64(media.size()));
                        QByteArray headers = offset ? "HTTP/1.1 206 Partial Content\r\n" : "HTTP/1.1 200 OK\r\n";
                        headers += "Content-Type: video/mp4\r\nAccept-Ranges: bytes\r\nContent-Length: " + QByteArray::number(media.size() - offset) + "\r\n";
                        if (offset) headers += "Content-Range: bytes " + QByteArray::number(offset) + "-" + QByteArray::number(media.size()-1) + "/" + QByteArray::number(media.size()) + "\r\n";
                        headers += "Connection: close\r\n\r\n";
                        socket->write(headers);
                        if (!request.startsWith("HEAD ")) socket->write(media.mid(offset));
                        socket->disconnectFromHost();
                    });
                    QObject::connect(socket, &QTcpSocket::disconnected, socket, &QObject::deleteLater);
                }
            });
            const QString url = QStringLiteral("http://127.0.0.1:%1/sample.mp4").arg(server.serverPort());
            QApplication::clipboard()->setText(url);
            click(window, named(window, "pasteButton"));
            auto waitFor = [](const std::function<bool()> &predicate, const char *message) {
                QElapsedTimer timer;
                timer.start();
                while (!predicate() && timer.elapsed() < 30000) QTest::qWait(100);
                check(predicate(), message);
            };
            waitFor([&]() { return backend->state["manual"].toObject()["state"] == "ready"; }, "Real media parsing did not reach ready");
            capture("real-engine-manual");
            click(window, named(window, "downloadButton"));
            waitFor([&]() { const auto jobs = backend->state["jobs"].toArray(); return !jobs.isEmpty() && jobs.first().toObject()["state"] == "complete"; }, "Real UI download did not complete");
            const auto job = backend->state["jobs"].toArray().first().toObject();
            const QString path = job["path"].toString();
            check(QDir::fromNativeSeparators(path).startsWith(profile.path() + "/downloads/") && QFileInfo(path).size() > 0, "Download escaped isolated output or is empty");
            QProcess probe;
            probe.start("ffprobe", {"-v", "error", "-show_entries", "stream=codec_type", "-of", "json", path});
            check(probe.waitForFinished(10000) && probe.exitCode() == 0, "Downloaded file failed ffprobe");
            check(QJsonDocument::fromJson(probe.readAllStandardOutput()).object()["streams"].toArray().size() == 2, "Downloaded video/audio streams missing");
            check(job["thumbnail"].toString().startsWith("file:"), "Completed local video has no thumbnail");
            evaluate("ui.activeTab = 'All'");
            QQuickItem *previewRow = nullptr;
            waitFor([&]() {
                rows.clear(); checkRows(list);
                for (auto *row : rows) if (engine.newQObject(row).property("modelData").property("id").toString() == job["id"].toString()) previewRow = row;
                return previewRow != nullptr;
            }, "Completed row did not appear");
            waitFor([&]() { return named(previewRow, "downloadThumbnail")->property("status").toInt() == 1; }, "Video thumbnail did not render");
            auto *playButton = qobject_cast<QQuickItem *>(named(previewRow, "thumbnailAction"));
            const QPoint playPoint = playButton->mapToScene(QPointF(playButton->width()/2, playButton->height()/2)).toPoint();
            check(QRect(QPoint(), window->size()).contains(playPoint), "Play hover target is outside the window");
            check(list->contains(list->mapFromScene(playPoint)), "Play hover target is clipped by its list");
            auto moveHover = [&](const QPoint &point) {
                QTest::mouseMove(window, point);
                // Cursor changes need event delivery, not a new scene frame after grabWindow.
                QTest::qWait(50);
            };
            const QPoint besidePlay = playPoint + QPoint(playButton->width(), 0);
            moveHover(besidePlay);
            check(!playButton->property("hovered").toBool() && window->cursor().shape() == Qt::ArrowCursor, "Play cursor baseline is not an unhovered arrow");
            moveHover(playPoint);
            check(playButton->property("hovered").toBool(), "Play hover event did not reach its target");
            check(window->cursor().shape() == Qt::PointingHandCursor, "Play hover cursor did not become a pointing hand");
            capture("real-engine-complete");
            moveHover(besidePlay);
            check(!playButton->property("hovered").toBool() && window->cursor().shape() == Qt::ArrowCursor, "Play cursor did not reset after leaving the button");
            QApplication::clipboard()->setText(url + "?second=1");
            QTest::keySequence(window, QKeySequence(QKeySequence::Paste));
            waitFor([&]() { return backend->state["manual"].toObject()["state"] == "ready"; }, "Second media parsing did not reach ready");
            click(window, named(window, "downloadButton"));
            waitFor([&]() {
                const auto jobs = backend->state["jobs"].toArray();
                return jobs.size() == 2 && jobs.first().toObject()["state"] == "complete" && jobs.last().toObject()["state"] == "complete";
            }, "Second download did not complete");
            backend->stopEngine();
            backend->startEngine(driver, profile.path());
            check(backend->state["jobs"].toArray().first().toObject()["state"] == "complete", "Completed download did not survive restart");
            check(window->color() == QColor("#292929"), "Dark theme did not survive download/restart");
            check(backend->state["settings"].toObject()["speedLimit"].toInt() == 0, "Temporary speed limit did not reset on restart");
            QTest::qWait(200);
            rows.clear();
            checkRows(list);
            check(rows.size() == 2, "Completed rows missing after restart");
            QQuickItem *completedRow = nullptr;
            for (auto *row : rows) if (engine.newQObject(row).property("modelData").property("id").toString() == job["id"].toString()) completedRow = row;
            check(completedRow != nullptr, "Original completed row not found");
            QTest::mouseClick(window, Qt::RightButton, Qt::NoModifier, completedRow->mapToScene(QPointF(completedRow->width()/2, completedRow->height()/2)).toPoint());
            QTest::qWait(150);
            check(named(window, "downloadContextMenu")->property("visible").toBool(), "Right click did not open context menu");
            click(window, named(window, "copyLinkAction"));
            check(QApplication::clipboard()->text() == url, "Copy Link did not copy the real source URL");
            evaluate("ui.confirmRemove()");
            QTest::qWait(100);
            auto *remove = named(window, "removeDialog");
            auto *deleteFiles = named(window, "deleteFilesCheckbox");
            check(!deleteFiles->property("checked").toBool(), "Delete files must default to off");
            deleteFiles->setProperty("checked", true);
            auto removeFooter = engine.newQObject(remove).property("footer");
            auto *cancel = removeFooter.property("standardButton").call({int(QDialogButtonBox::Cancel)}).toQObject();
            click(window, cancel);
            check(QFileInfo::exists(path) && backend->state["jobs"].toArray().size() == 2, "Cancel changed history or deleted the file");
            evaluate("ui.confirmRemove()");
            QTest::qWait(100);
            check(!deleteFiles->property("checked").toBool(), "Delete-files checkbox leaked from previous dialog");
            evaluate("ui.selectedIds = []");
            auto *confirm = removeFooter.property("standardButton").call({int(QDialogButtonBox::Ok)}).toQObject();
            click(window, confirm);
            waitFor([&]() { return backend->state["jobs"].toArray().size() == 1; }, "Removing one entry also removed its neighbor");
            check(backend->lastAction["ids"].toArray().size() == 1, "Confirmation dispatched an unscoped removal");
            check(QFileInfo::exists(path), "Removing history without the checkbox deleted the file");
            check(backend->state["jobs"].toArray().first().toObject()["id"] != job["id"], "Wrong history entry was removed");
            invoke(remove, "accept");
            check(backend->state["jobs"].toArray().size() == 1, "Repeated confirmation removed another entry");
            capture("real-engine-history-preserved");
            backend->dispatch("{\"action\":\"settings\",\"values\":{\"smartMode\":true}}");
            QApplication::clipboard()->setText(url + "?smart=first");
            click(window, named(window, "pasteButton"));
            check(backend->state["jobs"].toArray().size() == 2 && backend->state["manual"].isNull(), "Smart paste has no immediate queue row");
            QApplication::clipboard()->setText(url + "?smart=second");
            QTest::keySequence(window, QKeySequence(QKeySequence::Paste));
            QTest::qWait(100);
            check(backend->state["jobs"].toArray().size() == 3, "Second smart clipboard paste lost a source");
            waitFor([&]() {
                const auto jobs = backend->state["jobs"].toArray();
                for (const auto &value : jobs) if (value.toObject()["state"] != "complete") return false;
                return jobs.size() == 3;
            }, "Smart clipboard downloads did not complete");
            capture("real-engine-smart-paste");
            backend->stopEngine();
            qInfo("Real Engine: theme/restart, validation, settings, local UI download, ffprobe, durable history, context copy and safe removal passed.");
        }
        check(warnings.isEmpty(), warnings.join('\n'));
        qInfo("Appearance: themes, 5 settings tabs, 9 dialogs, standard Close, browser modes, menus, narrow window and contrast passed.");
        return 0;
    } catch (const std::exception &error) {
        qCritical("Appearance check failed: %s", error.what());
        return 1;
    }
}

#include "appearance.moc"
