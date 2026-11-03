pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Controls
import QtQuick.Dialogs
import QtQuick.Layouts

Dialog {
    id: root

    required property var app
    property int tab: 0

    objectName: "settingsDialog"
    title: qsTr("Preferences")
    modal: true
    anchors.centerIn: parent
    width: Math.min(850, parent.width - 50)
    height: Math.min(640, parent.height - 50)
    standardButtons: Dialog.Close

    FolderDialog {
        id: outputFolder

        title: qsTr("Save To")
        onAccepted: root.app.setting("path", root.app.nativePath(selectedFolder))
    }

    component Toggle: Switch {
        required property string settingKey

        Layout.fillWidth: true
        checked: root.app.settings[settingKey] === true
        onToggled: root.app.setting(settingKey, checked)
        Accessible.name: text
    }

    component Choice: RowLayout {
        required property string settingKey
        required property string label
        property var choices: []

        Layout.fillWidth: true
        spacing: 20

        Label {
 textFormat: Text.PlainText;
            text: label
            Layout.fillWidth: true
            wrapMode: Text.Wrap
        }

        ComboBox {
            Layout.preferredWidth: 210
            model: ((root.app.state.settings_options || {})[settingKey] || choices)
            currentIndex: Math.max(0, model.indexOf(root.app.optionText(settingKey, root.app.settings[settingKey])))
            onActivated: root.app.setting(settingKey, currentText)
            Accessible.name: label
        }
    }

    onOpened: {
        proxyHost.text = app.settings.proxyHost || "";
        proxyPort.text = String(app.settings.proxyPort || "");
        proxyLogin.text = app.settings.proxyLogin || "";
        proxyPassword.clear();
    }
    contentItem: RowLayout {
        spacing: 0

        ListView {
            id: navigation

            Layout.preferredWidth: 158
            Layout.fillHeight: true
            model: [qsTr("General"), qsTr("Advanced"), qsTr("Connection"), qsTr("Notification"), qsTr("Account")]
            currentIndex: root.tab

            delegate: ItemDelegate {
                required property int index
                required property string modelData

                width: navigation.width
                height: 52
                text: modelData
                highlighted: root.tab === index
                onClicked: root.tab = index
            }
        }

        Rectangle {
            implicitWidth: 1
            Layout.fillHeight: true
            color: "#ddd"
        }

        ScrollView {
            Layout.fillWidth: true
            Layout.fillHeight: true
            clip: true
            contentWidth: availableWidth

            ColumnLayout {
                width: parent.width
                spacing: 15

                ColumnLayout {
                    Layout.fillWidth: true
                    Layout.margins: 20
                    spacing: 18
                    visible: root.tab === 0

                    Choice {
                        label: qsTr("Language")
                        settingKey: "language"
                        choices: ["en", "ru"]
                    }

                    Label {
 textFormat: Text.PlainText;
                        text: qsTr("Restart the application after changing the interface language.")
                        color: "#888"
                        font.pixelSize: 12
                        wrapMode: Text.Wrap
                        Layout.fillWidth: true
                    }

                    Rectangle {
                        Layout.fillWidth: true
                        implicitHeight: 1
                        color: "#e8e8e8"
                    }

                    Toggle {
                        text: qsTr("Run VideoDock in background when it is closed")
                        settingKey: "systemTrayIconVisible"
                        enabled: (root.app.state.capabilities || {}).tray !== false
                    }

                    Toggle {
                        text: qsTr("Start automatically when the system is started")
                        settingKey: "autoStart"
                        enabled: (root.app.state.capabilities || {}).auto_start !== false
                    }

                    Toggle {
                        text: qsTr("Install updates automatically")
                        settingKey: "autoUpdate"
                    }

                    Toggle {
                        text: qsTr("Install Beta versions")
                        settingKey: "betaAccessEnabled"
                    }

                    Label {
 textFormat: Text.PlainText;
                        text: qsTr("Beta versions may be less stable and offer early access to new features.")
                        wrapMode: Text.Wrap
                        color: "#888"
                        Layout.fillWidth: true
                    }
                }

                ColumnLayout {
                    Layout.fillWidth: true
                    Layout.margins: 20
                    spacing: 13
                    visible: root.tab === 1

                    Label {
 textFormat: Text.PlainText;
                        text: qsTr("Download Preferences")
                        font.bold: true
                        font.pixelSize: 18
                    }

                    Choice {
                        label: qsTr("Simultaneous Downloads")
                        settingKey: "simultaneousDownloads"
                        choices: ["1", "2", "4", "6", "8"]
                    }

                    Label {
 textFormat: Text.PlainText;
                        text: qsTr("High intensity can lead to a temporary rate limit from the service.")
                        color: "#888"
                        font.pixelSize: 12
                        wrapMode: Text.Wrap
                        Layout.fillWidth: true
                    }

                    Choice {
                        label: qsTr("Subscription Updates")
                        settingKey: "subscriptionUpdatePeriod"
                        choices: ["5 min", "15 min", "30 min", "1 hour", "3 hours", "6 hours", "12 hours", "1 day"]
                    }

                    Repeater {
                        model: [
                            {
                                "key": "preventSleepEnabled",
                                "label": qsTr("Prevent system sleep until downloads are finished")
                            },
                            {
                                "key": "separateFolderCreationEnabled",
                                "label": qsTr("Create subdirectories for playlists, channels and search results")
                            },
                            {
                                "key": "numerationEnabled",
                                "label": qsTr("Numerate files in playlists, channels and search results")
                            },
                            {
                                "key": "skipDuplicates",
                                "label": qsTr("Skip duplicates in playlists and search results")
                            },
                            {
                                "key": "m3uEnabled",
                                "label": qsTr("Generate .m3u files for playlists and search results")
                            },
                            {
                                "key": "embedSubtitles",
                                "label": qsTr("Embed subtitles in the video file if possible")
                            },
                            {
                                "key": "tagsEnabled",
                                "label": qsTr("Search audio tags from the track title")
                            },
                            {
                                "key": "removeCompletedDownloads",
                                "label": qsTr("Remove downloaded items from the list")
                            },
                            {
                                "key": "recommendationServiceEnabled",
                                "label": qsTr("Submit download statistics")
                            },
                            {
                                "key": "aiProcessingAllowed",
                                "label": qsTr("Show Media Tools in the downloads list")
                            },
                            {
                                "key": "displayAllCodecs",
                                "label": qsTr("Display AV1/VP9 even when a more compatible codec is available")
                            }
                        ]

                        Toggle {
                            required property var modelData

                            settingKey: modelData.key
                            text: modelData.label
                        }
                    }

                    Rectangle {
                        Layout.fillWidth: true
                        implicitHeight: 1
                        color: "#ddd"
                    }

                    Label {
 textFormat: Text.PlainText;
                        text: qsTr("Smart Mode")
                        font.bold: true
                        font.pixelSize: 18
                    }

                    Choice {
                        label: qsTr("Download")
                        settingKey: "mediaType"
                        choices: ["video", "audio"]
                    }

                    Choice {
                        label: qsTr("Quality")
                        settingKey: "quality"
                        choices: ["Best", "8K", "4K", "1440p", "1080p", "720p", "480p", "360p", "320 kbps", "256 kbps", "128 kbps"]
                    }

                    Choice {
                        label: qsTr("Container")
                        settingKey: "container"
                        choices: ["MP4", "MKV", "MP3", "M4A", "OGG"]
                    }

                    Choice {
                        label: qsTr("Video codec")
                        settingKey: "videoCodec"
                        choices: ["Any", "H264", "H265", "VP9", "AV1"]
                    }

                    Choice {
                        label: qsTr("Audio codec")
                        settingKey: "audioCodec"
                        choices: ["Any", "AAC", "MP3", "Opus", "Vorbis"]
                    }

                    Choice {
                        label: qsTr("Frame Rate")
                        settingKey: "frameRate"
                        choices: ["Any", "Maximum", "60", "30", "25", "24"]
                    }

                    Choice {
                        label: qsTr("VR")
                        settingKey: "videoVR"
                        choices: ["Any", "2D", "3D", "360"]
                    }

                    Label {
 textFormat: Text.PlainText;
                        text: qsTr("Subtitle languages (comma separated)")
                        color: "#666"
                    }

                    TextField {
                        Layout.fillWidth: true
                        text: (root.app.settings.subtitles || []).join(", ")
                        placeholderText: "en, ru"
                        onEditingFinished: root.app.setting("subtitles", text.split(",").map(function (s) {
                            return s.trim();
                        }).filter(Boolean))
                    }

                    Label {
 textFormat: Text.PlainText;
                        text: qsTr("Audio track languages (comma separated)")
                        color: "#666"
                    }

                    TextField {
                        Layout.fillWidth: true
                        text: (root.app.settings.audioTracks || []).join(", ")
                        placeholderText: "en, ru"
                        onEditingFinished: root.app.setting("audioTracks", text.split(",").map(function (s) {
                            return s.trim();
                        }).filter(Boolean))
                    }

                    Button {
                        text: qsTr("Save To: ") + (root.app.settings.path || qsTr("Choose Folder…"))
                        Layout.fillWidth: true
                        onClicked: outputFolder.open()
                    }
                }

                ColumnLayout {
                    Layout.fillWidth: true
                    Layout.margins: 20
                    spacing: 15
                    visible: root.tab === 2

                    Choice {
                        label: qsTr("Speed")
                        settingKey: "speedLimit"
                        choices: ["Unlimited", "50 kbps", "100 kbps", "250 kbps", "500 kbps", "1 Mbps", "2 Mbps", "4 Mbps", "10 Mbps", "25 Mbps", "50 Mbps"]
                    }

                    Label {
 textFormat: Text.PlainText;
                        text: qsTr("Limit download speed on a slow connection. This temporary limit resets on relaunch.")
                        wrapMode: Text.Wrap
                        color: "#888"
                        Layout.fillWidth: true
                    }

                    Rectangle {
                        Layout.fillWidth: true
                        implicitHeight: 1
                        color: "#ddd"
                    }

                    Toggle {
                        text: qsTr("Enable Proxy")
                        settingKey: "proxyEnabled"
                    }

                    GridLayout {
                        columns: 2
                        rowSpacing: 12
                        columnSpacing: 18
                        Layout.fillWidth: true

                        Label {
 textFormat: Text.PlainText;
                            text: qsTr("Proxy Type")
                        }

                        ComboBox {
                            id: proxyType

                            Layout.fillWidth: true
                            model: ["HTTPS", "SOCKS5", "SOCKS4"]
                            currentIndex: Math.max(0, model.indexOf(root.app.settings.proxyType || "HTTPS"))
                        }

                        Label {
 textFormat: Text.PlainText;
                            text: qsTr("Host")
                        }

                        TextField {
                            id: proxyHost

                            Layout.fillWidth: true
                            text: root.app.settings.proxyHost || ""
                            placeholderText: "127.0.0.1"
                            selectByMouse: true
                        }

                        Label {
 textFormat: Text.PlainText;
                            text: qsTr("Port")
                        }

                        TextField {
                            id: proxyPort

                            Layout.fillWidth: true
                            text: String(root.app.settings.proxyPort || "")
                            placeholderText: "1080"

                            validator: IntValidator {
                                bottom: 1
                                top: 65535
                            }
                        }

                        Label {
 textFormat: Text.PlainText;
                            text: qsTr("Login")
                        }

                        TextField {
                            id: proxyLogin

                            Layout.fillWidth: true
                            text: root.app.settings.proxyLogin || ""
                            selectByMouse: true
                        }

                        Label {
 textFormat: Text.PlainText;
                            text: qsTr("Password")
                        }

                        TextField {
                            id: proxyPassword

                            Layout.fillWidth: true
                            echoMode: TextInput.Password
                            placeholderText: root.app.settings.proxyPasswordSet ? qsTr("Password saved") : ""
                        }
                    }

                    Button {
                        text: qsTr("Save Proxy Settings")
                        highlighted: true
                        Layout.alignment: Qt.AlignRight
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

                            var result = root.app.send({
                                "action": "settings",
                                "values": values
                            });
                            if (!result.error)
                                proxyPassword.clear();
                        }
                    }
                }

                ColumnLayout {
                    Layout.fillWidth: true
                    Layout.margins: 20
                    spacing: 15
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
                                "label": qsTr("Ask to confirm exit with incomplete downloads")
                            },
                            {
                                "key": "askDownloadChannel",
                                "label": qsTr("Suggest the channel after downloading several videos")
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

                        Toggle {
                            required property var modelData

                            settingKey: modelData.key
                            text: modelData.label
                        }
                    }
                }

                ColumnLayout {
                    Layout.fillWidth: true
                    Layout.margins: 24
                    spacing: 18
                    visible: root.tab === 4

                    Label {
 textFormat: Text.PlainText;
                        text: "YouTube"
                        font.pixelSize: 24
                        font.bold: true
                    }

                    Label {
 textFormat: Text.PlainText;
                        text: (root.app.state.account || {}).logged_in ? qsTr("Signed in as %1").arg(root.app.state.account.user_name || "YouTube") : qsTr("Sign in to access your Watch Later, Liked and other private playlists.")
                        Layout.fillWidth: true
                        wrapMode: Text.Wrap
                        font.pixelSize: 16
                    }

                    Label {
 textFormat: Text.PlainText;
                        text: qsTr("Google sign-in opens in the built-in browser. Your Google password is entered only on Google's page.")
                        Layout.fillWidth: true
                        wrapMode: Text.Wrap
                        color: "#888"
                    }

                    Button {
                        text: (root.app.state.account || {}).logged_in ? qsTr("Log Out") : qsTr("Log In")
                        highlighted: true
                        onClicked: root.app.send({
                            "action": "account",
                            "command": (root.app.state.account || {}).logged_in ? "logout" : "login"
                        })
                    }

                    Label {
 textFormat: Text.PlainText;
                        text: qsTr("Private content · YouTube library · Premium audio where available")
                        wrapMode: Text.Wrap
                        Layout.fillWidth: true
                        color: "#888"
                    }
                }
            }
        }
    }
}
