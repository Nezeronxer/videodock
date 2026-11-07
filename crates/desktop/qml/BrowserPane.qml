pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Controls
import QtQuick.Controls.impl
import QtQuick.Layouts
import QtQuick.Window
import QtWebEngine
import Qt5Compat.GraphicalEffects

Rectangle {
    id: root

    required property var app
    property bool home: true
    property string targetUrl: ""
    readonly property var web: webLoader.item
    readonly property bool compact: !app.browserVisible && app.hasMedia
    readonly property bool proxyBlocked: root.app.state.browserProxySupported === false
    readonly property string proxyMessage: root.app.state.browserProxyRestartRequired ? qsTr("Restart VideoDock to apply the browser proxy settings.") : qsTr("The browser supports HTTP proxies and SOCKS5 without authentication. Downloads support the configured proxy.")
    property var services: [
        {
            "name": "YouTube",
            "icon": "youtube",
            "url": "https://www.youtube.com"
        },
        {
            "name": "SoundCloud",
            "icon": "soundcloud",
            "url": "https://soundcloud.com"
        },
        {
            "name": "Facebook",
            "icon": "facebook",
            "url": "https://www.facebook.com"
        },
        {
            "name": "Vimeo",
            "icon": "vimeo",
            "url": "https://vimeo.com"
        },
        {
            "name": "TikTok",
            "icon": "tiktok",
            "url": "https://www.tiktok.com"
        },
        {
            "name": "Naver TV",
            "icon": "naver",
            "url": "https://tv.naver.com"
        },
        {
            "name": "Twitch",
            "icon": "twitch",
            "url": "https://www.twitch.tv"
        },
        {
            "name": "Dailymotion",
            "icon": "dailymotion",
            "url": "https://www.dailymotion.com"
        },
        {
            "name": "BiliBili",
            "icon": "bilibili",
            "url": "https://www.bilibili.com"
        },
        {
            "name": "Rumble",
            "icon": "rumble",
            "url": "https://rumble.com"
        }
    ]

    function go(value) {
        if (root.proxyBlocked) {
            root.app.localNotice = root.proxyMessage;
            return;
        }
        var input = String(value).trim();
        if (!input)
            return;

        var url = input.indexOf(" ") < 0 && input.indexOf(".") >= 0 ? (input.indexOf("://") >= 0 ? input : "https://" + input) : "https://www.youtube.com/results?search_query=" + encodeURIComponent(input);
        if (url.indexOf("https://") !== 0 && url.indexOf("http://") !== 0)
            return;

        app.browserVisible = true;
        var currentPage = root.web;
        targetUrl = url;
        address.text = url;
        home = false;
        if (currentPage)
            currentPage.url = url;
    }

    color: "transparent"
    radius: 2

    Rectangle {
        anchors.fill: parent
        anchors.topMargin: root.app.browserVisible ? 12 : 0
        color: root.app.theme.popup
        radius: 2
        layer.enabled: true
        layer.effect: DropShadow {
            color: Qt.rgba(0, 0, 0, 0.1)
            horizontalOffset: 0
            verticalOffset: 4
            samples: 33
        }
    }

    ColumnLayout {
        anchors.fill: parent
        anchors.topMargin: root.app.browserVisible ? 12 : 0
        spacing: 0

        RowLayout {
            Layout.fillWidth: true
            Layout.leftMargin: 16
            Layout.rightMargin: 8
            Layout.topMargin: 8
            Layout.bottomMargin: 8
            Layout.preferredHeight: 40
            Layout.maximumHeight: 40
            spacing: 0

            ToolButton {
                objectName: "browserHomeButton"
                Layout.preferredWidth: 28
                Layout.preferredHeight: 40
                text: qsTr("Home")
                display: AbstractButton.IconOnly
                palette.button: "transparent"
                icon.source: root.app.asset("home.png", "icons/home.svg")
                icon.width: 20
                icon.height: 20
                Accessible.name: qsTr("Home")
                enabled: !root.home
                onClicked: {
                    root.home = true;
                    root.targetUrl = "";
                    address.clear();
                    root.app.browserVisible = true;
                }
            }

            ToolButton {
                Layout.preferredWidth: 28
                Layout.preferredHeight: 40
                Layout.leftMargin: 24
                text: qsTr("Back")
                display: AbstractButton.IconOnly
                palette.button: "transparent"
                icon.source: root.app.asset("normal-left-arrow.png", "icons/back.svg")
                icon.width: 20
                icon.height: 20
                font.pixelSize: 25
                enabled: !!root.web && root.web.canGoBack
                Accessible.name: qsTr("Back")
                onClicked: web.goBack()
            }

            ToolButton {
                Layout.preferredWidth: 28
                Layout.preferredHeight: 40
                Layout.leftMargin: 24
                text: qsTr("Forward")
                display: AbstractButton.IconOnly
                palette.button: "transparent"
                icon.source: root.app.asset("normal-right-arrow.png", "icons/forward.svg")
                icon.width: 20
                icon.height: 20
                font.pixelSize: 25
                enabled: !!root.web && root.web.canGoForward
                Accessible.name: qsTr("Forward")
                onClicked: web.goForward()
            }

            TextField {
                id: address

                objectName: "browserAddress"
                Layout.fillWidth: true
                Layout.leftMargin: 24
                implicitHeight: 40
                leftPadding: 38
                rightPadding: 14
                placeholderText: !root.app.browserVisible ? qsTr("Click to show browser") : qsTr("Search by keywords")
                ColorImage {
                    objectName: "browserSearchIcon"
                    anchors.left: parent.left
                    anchors.leftMargin: 10
                    anchors.verticalCenter: parent.verticalCenter
                    width: 20
                    height: 20
                    source: root.app.asset("search.png", "icons/search.svg")
                    color: root.app.theme.text
                }
                onActiveFocusChanged: {
                    if (activeFocus)
                        root.app.browserVisible = true;
                }
                TapHandler {
                    onTapped: root.app.browserVisible = true
                }
                selectByMouse: true
                onAccepted: root.go(text)
            }

            Button {
                palette.dark: root.app.theme.accent
                palette.disabled.button: root.app.theme.dark ? "#292929" : "#f4f4f4"
                Layout.leftMargin: 8
                Layout.preferredWidth: address.text.indexOf(".") >= 0 && address.text.indexOf(" ") < 0 ? 120 : 140
                Layout.preferredHeight: 40
                text: address.text.indexOf(".") >= 0 && address.text.indexOf(" ") < 0 ? qsTr("Go") : qsTr("Search")
                enabled: address.text.length > 0
                onClicked: root.go(address.text)
            }

        }

        ProgressBar {

            palette.dark: root.app.theme.accent
            Layout.fillWidth: true
            from: 0
            to: 100
            value: root.web ? root.web.loadProgress : 0
            visible: !!root.web && root.web.loading
            implicitHeight: 3
        }

        Item {
            Layout.fillWidth: true
            Layout.fillHeight: true
            visible: !root.compact

            ColumnLayout {
                anchors.centerIn: parent
                visible: root.home && !root.proxyBlocked
                spacing: 16

                Label {
                    textFormat: Text.PlainText
                    visible: root.app.browserVisible
                    text: qsTr("Select the service below and enter your search query")
                    font.pixelSize: 28
                    color: root.app.theme.secondary
                    Layout.alignment: Qt.AlignHCenter
                }

                Label {
                    textFormat: Text.PlainText
                    visible: false
                    text: qsTr("Select a service and start searching")
                    color: root.app.theme.secondary
                    Layout.alignment: Qt.AlignHCenter
                }

                GridLayout {
                    id: serviceGrid
                    readonly property real scaleFactor: {
                        var screenArea = Screen.desktopAvailableWidth * Screen.desktopAvailableHeight;
                        var minArea = root.app.minimumWidth * root.app.minimumHeight;
                        return Math.min(2, Math.max(1, (root.width * root.height - minArea) / Math.max(1, screenArea - minArea) + 1));
                    }
                    columns: 5
                    rowSpacing: Math.ceil(10 * scaleFactor)
                    columnSpacing: rowSpacing
                    Layout.alignment: Qt.AlignHCenter

                    Repeater {
                        model: root.services

                        Button {
                            id: serviceItem
                            required property var modelData
                            Layout.preferredWidth: Math.ceil(80 * serviceGrid.scaleFactor)
                            Layout.preferredHeight: Layout.preferredWidth
                            text: modelData.name
                            readonly property string imageSource: root.app.asset((modelData.icon === "tiktok" ? "tik-tok" : modelData.icon) + "-icon.svg", "icons/service-" + modelData.icon + ".svg")
                            contentItem: Item {
                                Image {
                                    property int originalWidth: 0
                                    property int originalHeight: 0
                                    anchors.centerIn: parent
                                    source: serviceItem.imageSource
                                    sourceSize.width: originalWidth > 0 ? Math.ceil(serviceGrid.scaleFactor * originalWidth) : -1
                                    sourceSize.height: originalHeight > 0 ? Math.ceil(serviceGrid.scaleFactor * originalHeight) : -1
                                    onStatusChanged: {
                                        if (status === Image.Ready && originalWidth === 0) {
                                            originalWidth = implicitWidth;
                                            originalHeight = implicitHeight;
                                        }
                                    }
                                }
                            }
                            Accessible.name: text
                            ToolTip.text: text
                            ToolTip.visible: hovered
                            onClicked: root.go(modelData.url)
                            background: Rectangle {
                                radius: 40
                                color: serviceItem.down ? root.app.theme.selected : serviceItem.hovered ? root.app.theme.hover : "transparent"
                                border.width: serviceItem.visualFocus ? 2 : 0
                                border.color: root.app.green
                            }
                        }
                    }
                }
            }

            Label {
                anchors.centerIn: parent
                width: Math.min(parent.width - 48, 600)
                visible: root.proxyBlocked
                text: root.proxyMessage
                textFormat: Text.PlainText
                wrapMode: Text.WordWrap
                horizontalAlignment: Text.AlignHCenter
                color: root.app.theme.secondary
            }

            Loader {
                id: webLoader
                anchors.fill: parent
                active: !root.home && !root.proxyBlocked
                sourceComponent: WebEngineView {
                id: webView
                objectName: "webBrowser"
                Component.onCompleted: url = root.targetUrl
                backgroundColor: root.app.theme.window
                settings.forceDarkMode: root.app.theme.dark
                settings.javascriptCanOpenWindows: true
                settings.localContentCanAccessRemoteUrls: false
                settings.localContentCanAccessFileUrls: false
                onJavaScriptConsoleMessage: function (level, message, lineNumber, sourceID) {}
                onUrlChanged: {
                    if (!address.activeFocus)
                        address.text = String(url);
                }
                onNewWindowRequested: function (request) {
                    request.openIn(webView);
                }
                onLoadingChanged: function (request) {
                    if (request.status === WebEngineView.LoadFailedStatus)
                        root.app.localNotice = request.errorString;
                }
                onVisibleChanged: {
                    if (!visible && recentlyAudible)
                        runJavaScript("document.querySelectorAll('video,audio').forEach(function(media){media.pause()})");
                }
                }
            }
        }

        Rectangle {
            Layout.fillWidth: true
            implicitHeight: 1
            visible: !root.compact
            color: root.app.theme.border
        }
    }

    ToolButton {
        objectName: "hideBrowserButton"
        anchors.horizontalCenter: parent.horizontalCenter
        anchors.top: parent.top
        width: 36
        height: 15
        visible: root.app.browserVisible
        display: AbstractButton.IconOnly
        icon.source: root.app.asset("arrow-down.png", "icons/arrow-down.svg")
        icon.width: 20
        icon.height: 20
        Accessible.name: qsTr("Hide Browser")
        onClicked: root.app.browserVisible = false
        background: Rectangle { color: root.app.theme.popup; radius: 3 }
    }
    Button {
        anchors.horizontalCenter: parent.horizontalCenter
        anchors.bottom: parent.bottom
        anchors.bottomMargin: 16
        width: 210
        height: 56
        visible: root.app.browserVisible && !root.home && !!root.web
        text: qsTr("Download")
        icon.source: "icons/download.svg"
        icon.color: root.app.theme.accentText
        palette.buttonText: root.app.theme.accentText
        onClicked: root.app.parse(String(root.web.url))
        background: Rectangle {
            radius: 28
            color: root.app.green
            border.width: 2
            border.color: "white"
        }
    }
}
