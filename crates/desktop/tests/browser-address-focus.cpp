// Exercise production address/hide controls without the Rust app or WebEngine.
#include <QApplication>
#include <QFile>
#include <QQmlApplicationEngine>
#include <QQuickItem>
#include <QQuickStyle>
#include <QQuickWindow>
#include <QTest>
#include <stdexcept>

static void check(bool condition, const char *message) {
    if (!condition) throw std::runtime_error(message);
}

int main(int argc, char **argv) {
    QApplication application(argc, argv);
    QQuickStyle::setStyle("Basic");
    try {
        check(argc == 2, "Expected BrowserPane.qml");
        QFile source(QString::fromLocal8Bit(argv[1]));
        check(source.open(QIODevice::ReadOnly), "Cannot read production BrowserPane.qml");
        const QString body = QString::fromUtf8(source.readAll());
        auto control = [&](const QString &start, const QString &end) {
            const auto first = body.indexOf(start);
            check(first >= 0, "Production control not found");
            const auto last = body.indexOf(end, first + start.size());
            check(last >= 0, "Production control boundary not found");
            return body.mid(first, last + end.size() - first);
        };
        const QString address = control("            TextField {", "\n            }");
        const QString hide = control("    ToolButton {\n        objectName: \"hideBrowserButton\"", "\n    }");
        QQmlApplicationEngine engine;
        QStringList warnings;
        QObject::connect(&engine, &QQmlEngine::warnings, [&](const QList<QQmlError> &errors) {
            for (const auto &error : errors) warnings << error.toString();
        });
        const QString fixture = QStringLiteral(R"(
import QtQuick
import QtQuick.Controls
import QtQuick.Controls.impl
import QtQuick.Layouts
ApplicationWindow {
    visible: true; width: 420; height: 170
    QtObject {
        id: state
        objectName: "state"
        property bool browserVisible: false
        property var theme: ({text: "#222222", popup: "#ffffff"})
        function asset(a, b) { return "" }
    }
    Rectangle {
        id: root
        anchors.fill: parent
        property var app: state
        function go(value) {}
        RowLayout { anchors.centerIn: parent; width: parent.width - 30
%1
        }
%2
    }
})").arg(address, hide);
        engine.loadData(fixture.toUtf8());
        check(!engine.rootObjects().isEmpty(), "Production controls did not load");
        auto *window = qobject_cast<QQuickWindow *>(engine.rootObjects().first());
        auto *field = window->findChild<QQuickItem *>("browserAddress");
        auto *hideButton = window->findChild<QQuickItem *>("hideBrowserButton");
        auto *state = window->findChild<QObject *>("state");
        check(field && hideButton && state, "Fixture controls unavailable");
        window->requestActivate();
        check(QTest::qWaitForWindowActive(window, 3000), "Fixture window is not active");
        auto click = [&](QQuickItem *item) {
            const auto point = item->mapToScene(QPointF(item->width() / 2, item->height() / 2)).toPoint();
            QTest::mouseClick(window, Qt::LeftButton, Qt::NoModifier, point);
            QTest::qWait(100);
        };
        field->forceActiveFocus();
        QTest::qWait(50);
        check(state->property("browserVisible").toBool(), "Keyboard focus did not expand browser");
        click(hideButton);
        check(!state->property("browserVisible").toBool(), "Hide control did not minimize browser");
        // Cocoa buttons can leave keyboard focus on the address while minimizing.
        field->forceActiveFocus();
        state->setProperty("browserVisible", false);
        check(field->hasActiveFocus(), "Repeated-click precondition lost address focus");
        click(field);
        check(state->property("browserVisible").toBool(), "Already-focused address did not expand browser");
        field->setProperty("text", "selection fixture");
        const auto point = field->mapToScene(QPointF(field->property("leftPadding").toReal() + 12, field->height() / 2)).toPoint();
        QTest::mouseDClick(window, Qt::LeftButton, Qt::NoModifier, point);
        QTest::qWait(100);
        check(!field->property("selectedText").toString().isEmpty(), "Address double-click selection stopped working");
        QTest::keyClick(window, Qt::Key_X);
        check(field->property("text").toString() != "selection fixture", "Address keyboard editing stopped working");
        check(warnings.isEmpty(), qPrintable(warnings.join('\n')));
        qInfo("Browser address focus: repeated click, keyboard focus, selection and editing passed");
        return 0;
    } catch (const std::exception &error) {
        qCritical("Browser address focus failed: %s", error.what());
        return 1;
    }
}
