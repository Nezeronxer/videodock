pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import QtWebEngine

Rectangle {
    id: root

    required property var app
    property bool home: true
    property var services: [
        {
            "name": "YouTube",
            "url": "https://www.youtube.com",
            "mark": "▶",
            "color": "#e94335"
        },
        {
            "name": "SoundCloud",
            "url": "https://soundcloud.com",
            "mark": "☁",
            "color": "#f28532"
        },
        {
            "name": "Facebook",
            "url": "https://www.facebook.com",
            "mark": "f",
            "color": "#3476ce"
        },
        {
            "name": "Vimeo",
            "url": "https://vimeo.com",
            "mark": "v",
            "color": "#4cb7df"
        },
        {
            "name": "TikTok",
            "url": "https://www.tiktok.com",
            "mark": "♪",
            "color": "#252525"
        },
        {
            "name": "Naver TV",
            "url": "https://tv.naver.com",
            "mark": "N",
            "color": "#39bb78"
        },
        {
            "name": "Twitch",
            "url": "https://www.twitch.tv",
            "mark": "T",
            "color": "#9466d8"
        },
        {
            "name": "Dailymotion",
            "url": "https://www.dailymotion.com",
            "mark": "d",
            "color": "#3f8cd8"
        },
        {
            "name": "BiliBili",
            "url": "https://www.bilibili.com",
            "mark": "B",
            "color": "#53b9d4"
        },
        {
            "name": "Rumble",
            "url": "https://rumble.com",
            "mark": "▷",
            "color": "#80b93f"
        }
    ]

    function go(value) {
        var input = String(value).trim();
        if (!input)
            return;

        var url = input.indexOf(" ") < 0 && input.indexOf(".") >= 0 ? (input.indexOf("://") >= 0 ? input : "https://" + input) : "https://www.youtube.com/results?search_query=" + encodeURIComponent(input);
        if (url.indexOf("https://") !== 0 && url.indexOf("http://") !== 0)
            return;

        web.url = url;
        address.text = url;
        home = false;
    }

    color: "white"

    ColumnLayout {
        anchors.fill: parent
        spacing: 0

        RowLayout {
            Layout.fillWidth: true
            Layout.margins: 10
            spacing: 6

            ToolButton {
                text: "⌂"
                Accessible.name: qsTr("Home")
                onClicked: root.home = true
            }

            ToolButton {
                text: "‹"
                font.pixelSize: 25
                enabled: web.canGoBack
                Accessible.name: qsTr("Back")
                onClicked: web.goBack()
            }

            ToolButton {
                text: "›"
                font.pixelSize: 25
                enabled: web.canGoForward
                Accessible.name: qsTr("Forward")
                onClicked: web.goForward()
            }

            ToolButton {
                text: web.loading ? "×" : "↻"
                Accessible.name: qsTr("Reload")
                onClicked: web.loading ? web.stop() : web.reload()
            }

            TextField {
                id: address

                objectName: "browserAddress"
                Layout.fillWidth: true
                placeholderText: qsTr("Search by keywords or enter a URL")
                selectByMouse: true
                onAccepted: root.go(text)
            }

            Button {
                text: qsTr("Go")
                onClicked: root.go(address.text)
            }

            Button {
                text: qsTr("Download")
                highlighted: true
                enabled: !root.home
                onClicked: root.app.parse(String(web.url))
            }

            ToolButton {
                text: "×"
                Accessible.name: qsTr("Hide Browser")
                onClicked: root.app.browserVisible = false
            }
        }

        ProgressBar {
            Layout.fillWidth: true
            from: 0
            to: 100
            value: web.loadProgress
            visible: web.loading
            implicitHeight: 3
        }

        Item {
            Layout.fillWidth: true
            Layout.fillHeight: true

            ColumnLayout {
                anchors.centerIn: parent
                visible: root.home
                spacing: 16

                Label {
 textFormat: Text.PlainText;
                    text: qsTr("Find videos to download right in the app")
                    font.pixelSize: 23
                    color: "#777"
                    Layout.alignment: Qt.AlignHCenter
                }

                Label {
 textFormat: Text.PlainText;
                    text: qsTr("Select a service and start searching")
                    color: "#999"
                    Layout.alignment: Qt.AlignHCenter
                }

                GridLayout {
                    columns: 5
                    rowSpacing: 12
                    columnSpacing: 22
                    Layout.alignment: Qt.AlignHCenter

                    Repeater {
                        model: root.services

                        ColumnLayout {
                            required property var modelData

                            spacing: 4

                            Button {
                                text: modelData.mark
                                font.pixelSize: 28
                                font.bold: true
                                Layout.preferredWidth: 66
                                Layout.preferredHeight: 50
                                palette.buttonText: modelData.color
                                Accessible.name: modelData.name
                                onClicked: root.go(modelData.url)
                            }

                            Label {
 textFormat: Text.PlainText;
                                text: modelData.name
                                font.pixelSize: 11
                                color: "#888"
                                Layout.alignment: Qt.AlignHCenter
                            }
                        }
                    }
                }
            }

            WebEngineView {
                id: web

                objectName: "webBrowser"
                anchors.fill: parent
                visible: !root.home
                settings.javascriptCanOpenWindows: true
                settings.localContentCanAccessRemoteUrls: false
                settings.localContentCanAccessFileUrls: false
                onJavaScriptConsoleMessage: function (level, message, lineNumber, sourceID) {}
                onUrlChanged: {
                    if (!address.activeFocus)
                        address.text = String(url);
                }
                onNewWindowRequested: function (request) {
                    request.openIn(web);
                }
                onLoadingChanged: function (request) {
                    if (request.status === WebEngineView.LoadFailedStatus)
                        root.app.localNotice = request.errorString;
                }
            }
        }

        Rectangle {
            Layout.fillWidth: true
            implicitHeight: 1
            color: "#ddd"
        }
    }
}
