pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

Item {
    id: root

    required property var app
    property var rows: []

    ListView {
        id: list

        objectName: "downloadsList"
        anchors.fill: parent
        clip: true
        model: root.rows
        spacing: 1

        ScrollBar.vertical: ScrollBar {}

        delegate: Rectangle {
            id: row

            required property var modelData
            required property int index
            property bool selected: root.app.selectedIds.indexOf(modelData.id) >= 0

            width: list.width
            height: 106
            color: selected ? "#edf6e8" : hover.hovered ? "#f4f4f4" : "white"

            Rectangle {
                anchors.left: parent.left
                height: parent.height
                width: 3
                color: root.app.green
                visible: row.selected
            }

            HoverHandler {
                id: hover
            }

            TapHandler {
                acceptedButtons: Qt.LeftButton | Qt.RightButton
                onTapped: function (eventPoint, button) {
                    root.app.chooseJob(row.modelData, (point.modifiers & Qt.ControlModifier) !== 0);
                    if (button === Qt.RightButton)
                        contextMenu.popup();
                }
                onDoubleTapped: root.app.viewDetails(row.modelData)
            }

            RowLayout {
                anchors.fill: parent
                anchors.leftMargin: 17
                anchors.rightMargin: 18
                spacing: 14

                CheckBox {
                    checked: row.selected
                    onClicked: root.app.chooseJob(row.modelData, true)
                    Accessible.name: qsTr("Select %1").arg(row.modelData.title || "")
                }

                Rectangle {
                    Layout.preferredWidth: 112
                    Layout.preferredHeight: 70
                    color: "#e4e7e1"
                    radius: 3

                    Label {
 textFormat: Text.PlainText;
                        anchors.centerIn: parent
                        text: row.modelData.kind === "audio" ? "♫" : row.modelData.kind === "channel" ? "▤" : "▶"
                        font.pixelSize: 29
                        color: "#9ba592"
                    }

                    Image {
                        anchors.fill: parent
                        fillMode: Image.PreserveAspectCrop
                        asynchronous: true
                        source: {
                            var image = row.modelData.thumbnail || "";
                            return image.indexOf("file:") === 0 || image.indexOf("data:") === 0 ? image : "";
                        }
                    }
                }

                ColumnLayout {
                    Layout.fillWidth: true
                    spacing: 7

                    Label {
 textFormat: Text.PlainText;
                        text: row.modelData.title || row.modelData.url || qsTr("Untitled")
                        Layout.fillWidth: true
                        elide: Text.ElideRight
                        font.pixelSize: 15
                        font.bold: row.selected
                    }

                    RowLayout {
                        spacing: 10

                        Label {
 textFormat: Text.PlainText;
                            text: [row.modelData.format, row.modelData.quality, row.modelData.size].filter(Boolean).join(" · ")
                            color: "#888"
                            font.pixelSize: 12
                        }

                        Item {
                            Layout.fillWidth: true
                        }

                        Label {
 textFormat: Text.PlainText;
                            text: row.modelData.speed || ""
                            color: "#888"
                            font.pixelSize: 12
                        }
                    }

                    ProgressBar {
                        Layout.fillWidth: true
                        from: 0
                        to: 100
                        value: Number(row.modelData.progress || 0)
                        visible: ["downloading", "queued", "parsing", "processing", "paused"].indexOf(row.modelData.state) >= 0
                        indeterminate: row.modelData.state === "parsing"
                    }

                    Label {
 textFormat: Text.PlainText;
                        text: row.modelData.error || (root.app.statusTitle(row.modelData.state || "") + (row.modelData.remaining ? " · " + row.modelData.remaining : ""))
                        color: row.modelData.error ? "#b54537" : "#858585"
                        Layout.fillWidth: true
                        elide: Text.ElideRight
                        font.pixelSize: 12
                    }
                }

                ToolButton {
                    text: row.modelData.state === "paused" ? "▶" : "Ⅱ"
                    visible: ["queued", "downloading", "paused", "processing"].indexOf(row.modelData.state) >= 0
                    Accessible.name: row.modelData.state === "paused" ? qsTr("Resume") : qsTr("Pause")
                    onClicked: root.app.jobs(row.modelData.state === "paused" ? "resume" : "pause", [row.modelData.id])
                }

                ToolButton {
                    text: "♫"
                    visible: row.modelData.state === "complete"
                    Accessible.name: qsTr("Media Tools")
                    onClicked: {
                        root.app.chooseJob(row.modelData, false);
                        root.app.openMediaTools();
                    }
                }

                ToolButton {
                    text: "⋮"
                    font.pixelSize: 24
                    Accessible.name: qsTr("More actions")
                    onClicked: {
                        root.app.chooseJob(row.modelData, false);
                        contextMenu.popup();
                    }
                }
            }
        }
    }

    ColumnLayout {
        anchors.centerIn: parent
        width: Math.min(560, parent.width - 40)
        visible: root.rows.length === 0
        spacing: 16

        Rectangle {
            Layout.alignment: Qt.AlignHCenter
            Layout.preferredWidth: 90
            Layout.preferredHeight: 90
            radius: 45
            color: "#eff3ec"

            Label {
 textFormat: Text.PlainText;
                anchors.centerIn: parent
                text: root.app.activeTab === "Processed" ? "♫" : "↓"
                font.pixelSize: 54
                color: "#a6bc98"
            }
        }

        Label {
 textFormat: Text.PlainText;
            text: root.app.search ? qsTr("No media found") : root.app.activeTab === "Subscriptions" ? qsTr("Subscribe to channels and playlists") : root.app.activeTab === "Processed" ? qsTr("Processed items appear here") : qsTr("Download your first video")
            font.pixelSize: 24
            color: "#777"
            Layout.alignment: Qt.AlignHCenter
        }

        Label {
 textFormat: Text.PlainText;
            text: root.app.activeTab === "Subscriptions" ? qsTr("Paste a channel or playlist link and choose Subscribe to automatically download new videos.") : root.app.activeTab === "Processed" ? qsTr("Apply Media Tools to a completed download to extract voice or vocals.") : qsTr("Copy a video or song link, then click Paste Link. You can also find videos in the built-in browser.")
            wrapMode: Text.Wrap
            horizontalAlignment: Text.AlignHCenter
            Layout.fillWidth: true
            color: "#999"
            lineHeight: 1.4
        }

        Button {
            text: qsTr("Paste Link")
            highlighted: true
            Layout.alignment: Qt.AlignHCenter
            onClicked: root.app.pasteLink()
        }
    }

    Menu {
        id: contextMenu

        MenuItem {
            text: qsTr("Show Details")
            onTriggered: root.app.viewDetails(root.app.focusedJob)
        }

        MenuSeparator {}

        MenuItem {
            text: qsTr("Resume")
            onTriggered: root.app.jobs("resume")
        }

        MenuItem {
            text: qsTr("Pause")
            onTriggered: root.app.jobs("pause")
        }

        MenuItem {
            text: qsTr("Force")
            onTriggered: root.app.jobs("force")
        }

        MenuItem {
            text: qsTr("Redownload")
            onTriggered: root.app.jobs("redownload")
        }

        MenuItem {
            text: qsTr("Show in Folder")
            onTriggered: root.app.jobs("reveal")
        }

        MenuItem {
            text: qsTr("Open File")
            onTriggered: root.app.jobs("open")
        }

        MenuSeparator {}

        MenuItem {
            text: qsTr("Copy Link Address")
            onTriggered: root.app.jobs("copy_url")
        }

        MenuItem {
            text: qsTr("Open Links in Browser")
            onTriggered: root.app.jobs("open_url")
        }

        MenuItem {
            text: qsTr("Media Tools…")
            onTriggered: root.app.openMediaTools()
        }

        MenuItem {
            text: qsTr("Edit Subscription Settings")
            visible: root.app.activeTab === "Subscriptions"
            onTriggered: root.app.send({
                "action": "subscriptions",
                "command": "edit",
                "ids": root.app.selectedIds
            })
        }

        MenuItem {
            text: qsTr("Unsubscribe")
            visible: root.app.activeTab === "Subscriptions"
            onTriggered: root.app.send({
                "action": "subscriptions",
                "command": "unsubscribe",
                "ids": root.app.selectedIds
            })
        }

        MenuSeparator {}

        MenuItem {
            text: qsTr("Delete…")
            onTriggered: root.app.confirmRemove()
        }
    }
}
