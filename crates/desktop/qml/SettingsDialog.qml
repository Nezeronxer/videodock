pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Controls
import QtQuick.Dialogs
import QtQuick.Layouts

AppDialog {
    id: root

    property int tab: 0
    property bool proxyEditing: false
    property bool proxyEnableRequested: false
    readonly property var tabTitles: [qsTr("General"), qsTr("Advanced"), qsTr("Connection"), qsTr("Notification"), qsTr("Authorization")]
    readonly property int sidebarExtra: {
        let longest = 0;
        for (let label of tabTitles)
            longest = Math.max(longest, tabFontMetrics.advanceWidth(label) + 1);
        return Math.min(84, Math.max(0, 54 - 168 + longest));
    }

    objectName: "settingsDialog"
    title: tabTitles[tab]
    modal: true
    closePolicy: proxyEditing ? Popup.NoAutoClose : Popup.CloseOnEscape
    anchors.centerIn: parent
    width: Math.min(800 + sidebarExtra, parent.width - 32)
    height: Math.min(602, app.height - 90)
    padding: 0
    header: null
    footer: null
    Overlay.modal: Rectangle {
        color: Qt.rgba(0, 0, 0, 0.6)
    }
    background: Rectangle {
        color: root.app.theme.dark ? "#242424" : root.app.theme.window
        radius: 4
    }

    FontMetrics {
        id: tabFontMetrics
        font.pixelSize: 14
        font.weight: Font.DemiBold
    }

    FolderDialog {
        id: outputFolder

        title: qsTr("Save To")
        onAccepted: root.app.setting("path", root.app.nativePath(selectedFolder))
    }

    component Toggle: Switch {
        id: toggle
        palette.dark: root.app.green
        required property string settingKey

        Layout.fillWidth: true
        implicitHeight: Math.max(20, contentItem.implicitHeight)
        padding: 0
        rightPadding: 40
        checked: root.app.settings[settingKey] === true
        onToggled: root.app.setting(settingKey, checked)
        Accessible.name: text
        contentItem: Label {
            text: toggle.text
            color: toggle.enabled ? root.app.theme.text : root.app.theme.disabled
            font.pixelSize: 14
            wrapMode: Text.Wrap
            lineHeight: 20
            lineHeightMode: Text.FixedHeight
            verticalAlignment: Text.AlignVCenter
        }
        indicator: Rectangle {
            x: toggle.width - width
            y: (toggle.height - height) / 2
            width: 32
            height: 16
            radius: 8
            color: toggle.checked ? root.app.green : root.app.theme.dark ? "#666666" : "#d6d6d6"
            opacity: toggle.enabled ? 1 : 0.5
            Rectangle {
                x: toggle.checked ? 18 : 2
                y: 2
                width: 12
                height: 12
                radius: 6
                color: "white"
                Behavior on x {
                    NumberAnimation {
                        duration: 150
                    }
                }
            }
        }
        background: Item {}
    }

    component Choice: ColumnLayout {
        required property string settingKey
        required property string label
        property var choices: []
        property string iconName: ""
        property var labels: []

        Layout.preferredWidth: 268
        Layout.maximumWidth: 268
        Layout.fillWidth: false
        Layout.alignment: Qt.AlignLeft
        spacing: 8
        Label {
            textFormat: Text.PlainText
            text: label
            font.pixelSize: 14
            Layout.fillWidth: true
            wrapMode: Text.Wrap
        }

        ComboBox {
            id: optionChoice
            objectName: settingKey + "Choice"
            palette.dark: root.app.theme.secondary
            Layout.fillWidth: true
            implicitHeight: 40
            font.pixelSize: 14
            leftPadding: 12
            rightPadding: 32
            background: Rectangle {
                color: optionChoice.hovered ? root.app.theme.hover : root.app.theme.input
                border.color: optionChoice.activeFocus ? root.app.green : root.app.theme.border
                border.width: 1
            }
            indicator: Canvas {
                property color strokeColor: root.app.theme.text
                x: parent.width - width - 12
                y: (parent.height - height) / 2
                width: 12
                height: 8
                onStrokeColorChanged: requestPaint()
                onPaint: {
                    const ctx = getContext("2d");
                    ctx.clearRect(0, 0, width, height);
                    ctx.strokeStyle = strokeColor;
                    ctx.lineWidth = 1;
                    ctx.beginPath();
                    ctx.moveTo(1, 1);
                    ctx.lineTo(6, 6);
                    ctx.lineTo(11, 1);
                    ctx.stroke();
                }
            }
            model: ((root.app.state.settings_options || {})[settingKey] || choices)
            currentIndex: model.indexOf(root.app.optionText(settingKey, root.app.settings[settingKey]))
            displayText: labels[currentIndex] || root.app.optionLabel(currentIndex >= 0 ? currentText : root.app.optionText(settingKey, root.app.settings[settingKey]))
            delegate: ItemDelegate {
                required property int index
                required property string modelData
                width: optionChoice.width
                text: labels[index] || root.app.optionLabel(modelData)
                highlighted: index === optionChoice.highlightedIndex
            }
            onActivated: {
                const previous = root.app.settings[settingKey];
                root.app.setting(settingKey, currentText);
                if (settingKey === "language" && previous !== currentText)
                    languageRestart.open();
            }
            Accessible.name: label
        }
    }

    onTabChanged: settingsScroll.contentItem.contentY = 0
    function cancelProxy() {
        proxyEditing = false;
        proxyEnableRequested = false;
        proxyHost.text = app.settings.proxyHost || "";
        proxyPort.text = String(app.settings.proxyPort || "");
        proxyLogin.text = app.settings.proxyLogin || "";
        proxyPassword.clear();
        proxySwitch.checked = app.settings.proxyEnabled === true;
    }
    onOpened: {
        proxyEditing = false;
        proxyEnableRequested = false;
        proxyHost.text = app.settings.proxyHost || "";
        proxyPort.text = String(app.settings.proxyPort || "");
        proxyLogin.text = app.settings.proxyLogin || "";
        proxyPassword.clear();
    }

    Shortcut {
        sequence: "Escape"
        enabled: root.visible && root.proxyEditing
        onActivated: root.cancelProxy()
    }
    Shortcut {
        sequences: ["Return", "Enter"]
        enabled: root.visible && root.proxyEditing && proxySave.enabled
        onActivated: proxySave.clicked()
    }

    AppDialog {
        id: languageRestart
        objectName: "languageRestartDialog"
        app: root.app
        title: qsTr("Relaunch to change language")
        anchors.centerIn: parent
        width: Math.min(440, parent.width - 32)
        modal: true
        contentItem: ColumnLayout {
            spacing: 24
            Label {
                text: qsTr("Please quit the app and open it again.")
                Layout.fillWidth: true
                wrapMode: Text.Wrap
            }
            RowLayout {
                Layout.alignment: Qt.AlignRight
                Button {
                    text: qsTr("Close")
                    onClicked: languageRestart.close()
                }
                Button {
                    text: qsTr("Quit Now")
                    highlighted: true
                    palette.dark: root.app.green
                    onClicked: root.app.requestQuit()
                }
            }
        }
    }
    contentItem: RowLayout {
        spacing: 0

        Rectangle {
            Layout.preferredWidth: 200 + root.sidebarExtra
            Layout.fillHeight: true
            color: root.app.theme.dark ? "#2b2b2b" : "#f6f6f6"
            radius: 4
            ColumnLayout {
                anchors.top: parent.top
                anchors.left: parent.left
                anchors.right: parent.right
                anchors.margins: 16
                spacing: 0
                Repeater {
                    model: root.tabTitles
                    ItemDelegate {
                        required property int index
                        required property string modelData
                        Layout.fillWidth: true
                        Layout.preferredHeight: 40
                        objectName: "settingsTab" + index
                        font.pixelSize: 14
                        font.weight: root.tab === index ? Font.DemiBold : Font.Normal
                        text: modelData
                        icon.source: root.app.asset(["preferences.png", "filter.png", "connection-preferences.png", "notification-preferences.png", "account-preferences.png"][index], "icons/" + ["settings", "sliders", "connection", "notification", "account"][index] + ".svg")
                        icon.width: 20
                        icon.height: 20
                        spacing: 12
                        leftPadding: 8
                        palette.highlightedText: root.app.theme.text
                        highlighted: root.tab === index
                        background: Rectangle {
                            radius: 2
                            color: root.tab === index ? (root.app.theme.dark ? "#333333" : "#e8e8e8") : parent.hovered ? root.app.theme.hover : "transparent"
                        }
                        onClicked: root.tab = index
                    }
                }
            }
        }

        ColumnLayout {
            Layout.fillWidth: true
            Layout.fillHeight: true
            spacing: 0
            Item {
                Layout.fillWidth: true
                Layout.preferredHeight: 66
                Label {
                    x: 24
                    y: 24
                    width: parent.width - 80
                    objectName: "settingsHeading"
                    text: root.title
                    font.pixelSize: 18
                    font.letterSpacing: 0.7
                    elide: Text.ElideRight
                }
                ToolButton {
                    id: settingsClose
                    objectName: "settingsCloseButton"
                    anchors.right: parent.right
                    anchors.rightMargin: 18
                    anchors.verticalCenter: parent.verticalCenter
                    width: 32
                    height: 32
                    icon.source: "icons/close.svg"
                    icon.width: 16
                    icon.height: 16
                    icon.color: root.app.theme.text
                    display: AbstractButton.IconOnly
                    Accessible.name: qsTr("Close")
                    background: Rectangle {
                        radius: 16
                        color: settingsClose.hovered || settingsClose.activeFocus ? root.app.theme.hover : "transparent"
                    }
                    onClicked: root.close()
                }
            }
            ScrollView {
                id: settingsScroll
                Layout.fillWidth: true
                Layout.fillHeight: true
                clip: true
                contentWidth: availableWidth

                ColumnLayout {
                    width: parent.width
                    spacing: 15

                    ColumnLayout {
                        Layout.fillWidth: true
                        Layout.leftMargin: 24
                        Layout.rightMargin: 24
                        Layout.topMargin: 10
                        Layout.bottomMargin: 24
                        spacing: 0
                        visible: root.tab === 0

                        Choice {
                            label: qsTr("Language")
                            settingKey: "language"
                            iconName: "language"
                            choices: ["en", "ru"]
                        }

                        Rectangle {
                            Layout.topMargin: 24
                            Layout.fillWidth: true
                            implicitHeight: 1
                            color: root.app.theme.border
                        }

                        Toggle {
                            Layout.topMargin: 26
                            text: qsTr("Run VideoDock in background when it is closed")
                            settingKey: "systemTrayIconVisible"
                            enabled: (root.app.state.capabilities || {}).tray !== false
                        }

                        Toggle {
                            Layout.topMargin: 20
                            text: qsTr("Start automatically when the system is started")
                            settingKey: "autoStart"
                            enabled: (root.app.state.capabilities || {}).auto_start !== false
                        }

                        Toggle {
                            Layout.topMargin: 20
                            text: qsTr("Install updates automatically")
                            settingKey: "autoUpdate"
                        }

                        Toggle {
                            Layout.topMargin: 20
                            text: qsTr("Install Beta versions")
                            settingKey: "betaAccessEnabled"
                        }

                        Label {
                            textFormat: Text.PlainText
                            Layout.topMargin: 16
                            Layout.preferredWidth: 420
                            Layout.maximumWidth: 420
                            font.pixelSize: 12
                            lineHeight: 20
                            lineHeightMode: Text.FixedHeight
                            text: qsTr("Beta versions may be less stable than official ones though provide access to using new features before their initial release. Please report bugs and issues.")
                            wrapMode: Text.Wrap
                            color: root.app.theme.secondary
                            Layout.fillWidth: true
                        }
                    }

                    ColumnLayout {
                        Layout.fillWidth: true
                        Layout.leftMargin: 24
                        Layout.rightMargin: 24
                        Layout.topMargin: 4
                        Layout.bottomMargin: 20
                        spacing: 0
                        visible: root.tab === 1
                        Label {
                            text: qsTr("Simultaneous Downloads")
                            font.pixelSize: 14
                        }
                        Slider {
                            id: simultaneous
                            objectName: "simultaneousDownloadsSlider"
                            property var choices: (root.app.state.settings_options || {}).simultaneousDownloads || ["1", "2", "4", "6", "8"]
                            Layout.fillWidth: true
                            Layout.preferredHeight: 20
                            Layout.topMargin: 16
                            from: 0
                            to: Math.max(0, choices.length - 1)
                            stepSize: 1
                            snapMode: Slider.SnapAlways
                            padding: 0
                            value: Math.max(0, choices.indexOf(String(root.app.settings.simultaneousDownloads)))
                            Accessible.name: qsTr("Simultaneous Downloads") + ": " + choices[Math.round(value)]
                            onMoved: root.app.setting("simultaneousDownloads", choices[Math.round(value)])
                            background: Rectangle {
                                y: (simultaneous.height - height) / 2
                                width: simultaneous.availableWidth
                                height: 2
                                color: root.app.theme.border
                                Rectangle {
                                    width: simultaneous.visualPosition * parent.width
                                    height: 2
                                    color: root.app.green
                                }
                                Repeater {
                                    model: simultaneous.choices
                                    Rectangle {
                                        required property int index
                                        x: index * (simultaneous.availableWidth - width) / Math.max(1, simultaneous.choices.length - 1)
                                        y: -1
                                        width: 4
                                        height: 4
                                        radius: 2
                                        color: simultaneous.value >= index ? root.app.green : root.app.theme.disabled
                                    }
                                }
                            }
                            handle: Rectangle {
                                x: simultaneous.visualPosition * simultaneous.availableWidth - width / 2
                                y: (simultaneous.height - height) / 2
                                width: 12
                                height: 12
                                radius: 6
                                color: root.app.green
                            }
                        }
                        Item {
                            Layout.fillWidth: true
                            Layout.topMargin: 19
                            implicitHeight: 40
                            Repeater {
                                model: simultaneous.choices
                                Label {
                                    required property int index
                                    required property string modelData
                                    text: modelData + "\n" + [qsTr("Safe"), qsTr("Safe"), qsTr("Stable"), qsTr("Optimal"), qsTr("Risky")][index]
                                    font.pixelSize: 14
                                    lineHeight: 20
                                    lineHeightMode: Text.FixedHeight
                                    y: -10
                                    horizontalAlignment: index === 0 ? Text.AlignLeft : index === simultaneous.choices.length - 1 ? Text.AlignRight : Text.AlignHCenter
                                    x: index === 0 ? 0 : index === simultaneous.choices.length - 1 ? parent.width - width : parent.width * index / Math.max(1, simultaneous.choices.length - 1) - width / 2
                                }
                            }
                        }
                        Label {
                            Layout.topMargin: 12
                            Layout.preferredWidth: 360
                            text: qsTr("High intensity might increase overall download performance but it may also lead to temporary IP address ban by YouTube.")
                            lineHeight: 20
                            lineHeightMode: Text.FixedHeight
                            font.pixelSize: 12
                            color: root.app.theme.secondary
                            wrapMode: Text.Wrap
                        }
                        Choice {
                            Layout.topMargin: 32
                            Layout.preferredWidth: 200
                            Layout.maximumWidth: 200
                            label: qsTr("Subscription Updates")
                            settingKey: "subscriptionUpdatePeriod"
                            choices: ["Never", "5 min", "15 min", "30 min", "1 hour", "3 hours", "6 hours", "12 hours", "1 day"]
                        }
                        ColumnLayout {
                            Layout.fillWidth: true
                            Layout.topMargin: 34
                            spacing: 20
                            Repeater {
                                model: [
                                    {
                                        key: "preventSleepEnabled",
                                        label: qsTr("Prevent system sleep until downloads are finished")
                                    },
                                    {
                                        key: "displayAllCodecs",
                                        label: qsTr("Display AV1/VP9 even when a more compatible video codec is available")
                                    },
                                    {
                                        key: "separateFolderCreationEnabled",
                                        label: qsTr("Create subdirectories for downloaded playlists, channels and search results")
                                    },
                                    {
                                        key: "numerationEnabled",
                                        label: qsTr("Numerate files in playlists, channels and search results")
                                    },
                                    {
                                        key: "skipDuplicates",
                                        label: qsTr("Skip duplicates in playlists and search results")
                                    },
                                    {
                                        key: "m3uEnabled",
                                        label: qsTr("Generate .m3u files for downloaded playlists and search results")
                                    },
                                    {
                                        key: "embedSubtitles",
                                        label: qsTr("Embed subtitles in video file if possible")
                                    },
                                    {
                                        key: "tagsEnabled",
                                        label: qsTr("Search audio tags on the basis of track title")
                                    },
                                    {
                                        key: "removeCompletedDownloads",
                                        label: qsTr("Remove downloaded items from the list")
                                    },
                                    {
                                        key: "recommendationServiceEnabled",
                                        label: qsTr("Submit download statistics")
                                    },
                                    {
                                        key: "aiProcessingAllowed",
                                        label: qsTr("Show Media Tools in downloads list")
                                    }
                                ]
                                Toggle {
                                    required property var modelData
                                    settingKey: modelData.key
                                    text: modelData.label
                                    enabled: modelData.key !== "aiProcessingAllowed" || (root.app.state.capabilities || {}).processing !== false
                                }
                            }
                        }
                    }

                    ColumnLayout {
                        Layout.fillWidth: true
                        Layout.leftMargin: 24
                        Layout.rightMargin: 24
                        Layout.topMargin: 10
                        Layout.bottomMargin: 24
                        spacing: 0
                        visible: root.tab === 2

                        Choice {
                            label: qsTr("Speed")
                            settingKey: "speedLimit"
                            choices: ["Unlimited", "50 kbps", "100 kbps", "250 kbps", "500 kbps", "1 Mbps", "2 Mbps", "4 Mbps", "10 Mbps", "25 Mbps", "50 Mbps"]
                        }

                        Label {
                            textFormat: Text.PlainText
                            Layout.topMargin: 16
                            font.pixelSize: 12
                            lineHeight: 20
                            lineHeightMode: Text.FixedHeight
                            text: qsTr("You can limit the download speed if you have a slow Internet connection. Please note this setting will be reset with the app relaunch.")
                            wrapMode: Text.Wrap
                            color: root.app.theme.secondary
                            Layout.fillWidth: true
                        }

                        Rectangle {
                            Layout.topMargin: 24
                            Layout.bottomMargin: 24
                            Layout.fillWidth: true
                            implicitHeight: 1
                            color: root.app.theme.border
                        }

                        Rectangle {
                            Layout.fillWidth: true
                            implicitHeight: proxyColumn.implicitHeight
                            color: root.app.theme.dark ? "#333333" : "#f6f6f6"
                            radius: 2
                            ColumnLayout {
                                id: proxyColumn
                                width: parent.width
                                spacing: 0
                                RowLayout {
                                    Layout.fillWidth: true
                                    Layout.leftMargin: 12
                                    Layout.rightMargin: 12
                                    Layout.preferredHeight: 40
                                    spacing: 8
                                    Label {
                                        text: qsTr("Set proxy manually")
                                        font.pixelSize: 14
                                    }
                                    Item {
                                        Layout.fillWidth: true
                                    }
                                    Button {
                                        text: qsTr("Edit")
                                        flat: true
                                        visible: root.app.settings.proxyEnabled === true && !root.proxyEditing
                                        palette.windowText: root.app.green
                                        onClicked: root.proxyEditing = true
                                    }
                                    Label {
                                        visible: root.app.settings.proxyEnabled === true && !root.proxyEditing && !!root.app.settings.proxyHost
                                        text: (root.app.settings.proxyHost || "") + ":" + root.app.settings.proxyPort
                                        color: root.app.theme.secondary
                                        elide: Text.ElideRight
                                        Layout.maximumWidth: 160
                                    }
                                    Switch {
                                        id: proxySwitch
                                        objectName: "proxyEnabledToggle"
                                        Accessible.name: qsTr("Set proxy manually")
                                        checked: root.app.settings.proxyEnabled === true || root.proxyEnableRequested
                                        padding: 0
                                        implicitWidth: 32
                                        implicitHeight: 16
                                        onToggled: {
                                            if (root.app.settings.proxyHost)
                                                root.app.setting("proxyEnabled", checked);
                                            else if (checked) {
                                                root.proxyEnableRequested = true;
                                                root.proxyEditing = true;
                                            } else
                                                root.cancelProxy();
                                        }
                                        indicator: Rectangle {
                                            width: 32
                                            height: 16
                                            radius: 8
                                            color: proxySwitch.checked ? root.app.green : root.app.theme.dark ? "#666666" : "#d6d6d6"
                                            Rectangle {
                                                x: proxySwitch.checked ? 18 : 2
                                                y: 2
                                                width: 12
                                                height: 12
                                                radius: 6
                                                color: "white"
                                            }
                                        }
                                    }
                                }
                                Label {
                                    visible: !proxySwitch.checked
                                    text: qsTr("Automatic: system and VPN settings")
                                    color: root.app.theme.secondary
                                    Layout.fillWidth: true
                                    Layout.leftMargin: 12
                                    Layout.rightMargin: 12
                                    Layout.bottomMargin: 12
                                    wrapMode: Text.Wrap
                                }
                                ColumnLayout {
                                    visible: root.proxyEditing
                                    Layout.fillWidth: true
                                    Layout.margins: 12
                                    Layout.topMargin: 24
                                    spacing: 12
                                    GridLayout {
                                        columns: 2
                                        rowSpacing: 12
                                        columnSpacing: 18
                                        Layout.fillWidth: true

                                        Label {
                                            textFormat: Text.PlainText
                                            text: qsTr("Proxy Type")
                                        }

                                        ComboBox {
                                            id: proxyType
                                            palette.dark: root.app.theme.secondary

                                            Layout.preferredWidth: 258
                                            implicitHeight: 40
                                            model: ["HTTP", "HTTPS", "SOCKS5", "SOCKS4"]
                                            currentIndex: Math.max(0, model.indexOf(root.app.settings.proxyType || "HTTPS"))
                                        }

                                        Label {
                                            textFormat: Text.PlainText
                                            text: qsTr("Host")
                                        }

                                        TextField {
                                            id: proxyHost
                                            objectName: "proxyHost"

                                            Layout.preferredWidth: 258
                                            implicitHeight: 40
                                            text: root.app.settings.proxyHost || ""
                                            placeholderText: "127.0.0.1"
                                            selectByMouse: true
                                        }

                                        Label {
                                            textFormat: Text.PlainText
                                            text: qsTr("Port")
                                        }

                                        TextField {
                                            id: proxyPort
                                            objectName: "proxyPort"

                                            Layout.preferredWidth: 258
                                            implicitHeight: 40
                                            text: String(root.app.settings.proxyPort || "")
                                            placeholderText: "1080"

                                            validator: IntValidator {
                                                bottom: 1
                                                top: 65535
                                            }
                                        }

                                        Label {
                                            textFormat: Text.PlainText
                                            text: qsTr("Login")
                                        }

                                        TextField {
                                            id: proxyLogin

                                            Layout.preferredWidth: 258
                                            implicitHeight: 40
                                            text: root.app.settings.proxyLogin || ""
                                            selectByMouse: true
                                        }

                                        Label {
                                            textFormat: Text.PlainText
                                            text: qsTr("Password")
                                        }

                                        TextField {
                                            id: proxyPassword

                                            Layout.preferredWidth: 258
                                            implicitHeight: 40
                                            echoMode: TextInput.Password
                                            placeholderText: root.app.settings.proxyPasswordSet ? qsTr("Password saved") : ""
                                        }
                                    }

                                    RowLayout {
                                        Layout.fillWidth: true
                                        Layout.topMargin: 12
                                        spacing: 12
                                        Button {
                                            text: qsTr("Cancel")
                                            Layout.fillWidth: true
                                            implicitHeight: 40
                                            onClicked: root.cancelProxy()
                                        }

                                        Button {
                                            id: proxySave
                                            palette.dark: root.app.theme.accent
                                            text: qsTr("Save")
                                            objectName: "saveProxySettings"
                                            highlighted: true
                                            Layout.fillWidth: true
                                            implicitHeight: 40
                                            enabled: proxyHost.text.trim().length > 0 && proxyPort.acceptableInput && (proxyLogin.text.length === 0 || proxyPassword.text.length > 0 || root.app.settings.proxyPasswordSet)
                                            onClicked: {
                                                var values = {
                                                    "proxyType": proxyType.currentText,
                                                    "proxyHost": proxyHost.text.trim(),
                                                    "proxyPort": Number(proxyPort.text),
                                                    "proxyLogin": proxyLogin.text
                                                };
                                                if (proxyPassword.text.length)
                                                    values.proxyPassword = proxyPassword.text;

                                                if (root.proxyEnableRequested)
                                                    values.proxyEnabled = true;
                                                var result = root.app.send({
                                                    "action": "settings",
                                                    "values": values
                                                });
                                                if (!result.error) {
                                                    proxyPassword.clear();
                                                    root.proxyEditing = false;
                                                    root.proxyEnableRequested = false;
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }

                    ColumnLayout {
                        Layout.fillWidth: true
                        Layout.leftMargin: 24
                        Layout.rightMargin: 24
                        Layout.topMargin: 14
                        Layout.bottomMargin: 24
                        spacing: 20
                        visible: root.tab === 3

                        Repeater {
                            model: [
                                {
                                    "key": "notificationEnabled",
                                    "label": qsTr("Show notification when downloading finished")
                                },
                                {
                                    "key": "processingNotificationsEnabled",
                                    "label": qsTr("Show notification when processing completes")
                                },
                                {
                                    "key": "remindNotificationEnabled",
                                    "label": qsTr("Show reminders to download new content")
                                },
                                {
                                    "key": "similarVideosNotificationEnabled",
                                    "label": qsTr("Show similar video recommendations")
                                },
                                {
                                    "key": "askQuit",
                                    "label": qsTr("Ask to confirm the app exit if there are incomplete downloads")
                                },
                                {
                                    "key": "askDownloadChannel",
                                    "label": qsTr("Ask to download channel if multiple videos were downloaded from it")
                                },
                                {
                                    "key": "systemNotificationsEnabled",
                                    "label": qsTr("Show in Notification Center")
                                },
                                {
                                    "key": "notificationSound",
                                    "label": qsTr("Play notification sound")
                                }
                            ]

                            ColumnLayout {
                                required property var modelData
                                Layout.fillWidth: true
                                spacing: 0
                                Rectangle {
                                    visible: modelData.key === "askQuit" || modelData.key === "systemNotificationsEnabled"
                                    Layout.fillWidth: true
                                    Layout.topMargin: 6
                                    Layout.bottomMargin: 26
                                    implicitHeight: 1
                                    color: root.app.theme.border
                                }
                                Toggle {
                                    settingKey: modelData.key
                                    text: modelData.label
                                    enabled: modelData.key !== "processingNotificationsEnabled" || (root.app.state.capabilities || {}).processing !== false
                                }
                            }
                        }
                    }

                    ColumnLayout {
                        Layout.fillWidth: true
                        Layout.leftMargin: 24
                        Layout.rightMargin: 24
                        Layout.topMargin: 14
                        Layout.bottomMargin: 24
                        spacing: 0
                        visible: root.tab === 4
                        Label {
                            text: "YouTube"
                            font.pixelSize: 14
                        }
                        RowLayout {
                            Layout.fillWidth: true
                            Layout.topMargin: 8
                            Layout.bottomMargin: 24
                            spacing: 10
                            Image {
                                Layout.preferredWidth: 40
                                Layout.preferredHeight: 40
                                visible: !!(root.app.state.account || {}).logged_in
                                source: (root.app.state.account || {}).avatar_url || "icons/account.svg"
                                fillMode: Image.PreserveAspectFit
                            }
                            Label {
                                textFormat: Text.PlainText
                                text: (root.app.state.account || {}).logged_in ? qsTr("You're currently authorized as %1. If you want to change the account, log out and sign in with another one.").arg(root.app.state.account.user_name || "YouTube") : qsTr("Authorization with YouTube enables downloading age-restricted videos and private playlists (Watch Later, Liked, etc.). Log In is secure — credentials go only to Google and are never stored or accessed by us.")
                                Layout.fillWidth: true
                                color: root.app.theme.secondary
                                font.pixelSize: 12
                                lineHeight: 20
                                lineHeightMode: Text.FixedHeight
                                wrapMode: Text.Wrap
                            }
                            Button {
                                Layout.preferredWidth: (root.app.state.account || {}).logged_in ? implicitWidth : 173
                                Layout.alignment: Qt.AlignTop
                                Layout.topMargin: 5
                                implicitHeight: (root.app.state.account || {}).logged_in ? 20 : 40
                                text: (root.app.state.account || {}).logged_in ? qsTr("Log Out") : qsTr("Log In")
                                highlighted: !(root.app.state.account || {}).logged_in
                                flat: !!(root.app.state.account || {}).logged_in
                                palette.dark: root.app.green
                                onClicked: root.app.send({
                                    action: "account",
                                    command: (root.app.state.account || {}).logged_in ? "logout" : "login"
                                })
                            }
                        }
                        Rectangle {
                            Layout.fillWidth: true
                            Layout.bottomMargin: 24
                            implicitHeight: 1
                            color: root.app.theme.border
                        }
                        GridLayout {
                            Layout.fillWidth: true
                            columns: 3
                            rowSpacing: 24
                            columnSpacing: 18
                            Repeater {
                                model: [
                                    {
                                        icon: "account",
                                        label: qsTr("Private content downloads")
                                    },
                                    {
                                        icon: "browser",
                                        label: qsTr("YouTube library access")
                                    },
                                    {
                                        icon: "audio",
                                        label: qsTr("Premium audio downloads")
                                    },
                                    {
                                        icon: "check",
                                        label: qsTr("Secure Google sign-in")
                                    },
                                    {
                                        icon: "language",
                                        label: qsTr("Credentials stay on Google")
                                    }
                                ]
                                ColumnLayout {
                                    required property var modelData
                                    Layout.fillWidth: true
                                    Layout.preferredHeight: 72
                                    spacing: 12
                                    ToolButton {
                                        enabled: false
                                        display: AbstractButton.IconOnly
                                        icon.source: "icons/" + modelData.icon + ".svg"
                                        icon.width: 20
                                        icon.height: 20
                                        icon.color: root.app.theme.text
                                        padding: 0
                                        implicitWidth: 20
                                        implicitHeight: 20
                                        background: Item {}
                                    }
                                    Label {
                                        text: modelData.label
                                        Layout.fillWidth: true
                                        wrapMode: Text.WordWrap
                                        font.pixelSize: 14
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
