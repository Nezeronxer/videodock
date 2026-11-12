pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

Item {
    id: root

    required property var app
    property var rows: []
    property string firstVisibleId: ""
    onRowsChanged: {
        var next = rows.length ? String(rows[0].id) : "";
        if (next && next !== firstVisibleId && app.sortKey === "date" && app.descending)
            Qt.callLater(function () { list.positionViewAtBeginning(); });
        firstVisibleId = next;
    }

    function platformIcon(url) {
        return app.platformIcon(url);
    }

    function openCompleted(job) {
        if (job.state === "complete" && job.path)
            app.jobs("open", [job.id]);
    }
    function description(job, hovered) {
        if (job.error)
            return job.error;
        var duration = typeof job.duration === "number" ? Math.floor(job.duration / 60) + ":" + String(Math.floor(job.duration % 60)).padStart(2, "0") : job.duration;
        var metadata = [duration, job.size, job.format, job.quality, job.channel_title].filter(Boolean);
        if (job.state === "complete")
            return metadata.join(" · ");
        if (job.state === "parsing")
            return qsTr("retrieving information") + "…";
        var progress = Number(job.progress || 0);
        var status = job.state === "processing" ? qsTr("%1% converted").arg(progress) : qsTr("%1% downloaded").arg(progress);
        if (["queued", "downloading", "processing", "paused"].indexOf(job.state) >= 0)
            return [status, job.remaining ? qsTr("%1 left").arg(job.remaining) : "", job.state === "paused" ? "" : job.speed].concat(hovered ? metadata : []).filter(Boolean).join(" · ");
        return app.statusTitle(job.state || "");
    }

    ListView {
        id: list

        objectName: "downloadsList"
        anchors.fill: parent
        clip: true
        model: root.rows
        spacing: 0
        footer: Item { height: 34 }

        ScrollBar.vertical: ScrollBar {}

        delegate: Item {
            id: slot

            required property var modelData
            required property int index
            width: list.width
            height: 64

        // ListView owns the slot position; center the card independently inside it.
        Rectangle {
            id: row

            readonly property var modelData: slot.modelData
            readonly property int index: slot.index
            property bool selected: root.app.selectedIds.indexOf(modelData.id) >= 0
            readonly property bool current: root.app.focusedJob && root.app.focusedJob.id === modelData.id
            readonly property bool previousSelected: index > 0 && root.app.selectedIds.indexOf(root.rows[index - 1].id) >= 0
            readonly property bool nextSelected: index + 1 < root.rows.length && root.app.selectedIds.indexOf(root.rows[index + 1].id) >= 0
            readonly property bool nextCurrent: index + 1 < root.rows.length && root.app.focusedJob && root.app.focusedJob.id === root.rows[index + 1].id
            readonly property bool busy: ["queued", "parsing", "downloading", "processing"].indexOf(modelData.state) >= 0

            objectName: "downloadRow"
            width: Math.min(list.width - 34, 1080)
            anchors.horizontalCenter: parent.horizontalCenter
            height: 64
            radius: 2
            topLeftRadius: selected && previousSelected ? 0 : 2
            topRightRadius: topLeftRadius
            bottomLeftRadius: selected && nextSelected ? 0 : 2
            bottomRightRadius: bottomLeftRadius
            color: selected ? root.app.theme.selected : root.app.theme.toolbar

            Rectangle { anchors.left: parent.left; width: 1; height: parent.height; color: "#f8e6ab"; visible: row.selected || row.current }
            Rectangle { anchors.right: parent.right; width: row.selected ? 3 : 1; height: parent.height; color: "#f8e6ab"; visible: row.selected || row.current }
            Rectangle { anchors.top: parent.top; width: parent.width; height: 1; color: "#f8e6ab"; visible: row.current || (row.selected && !row.previousSelected) }
            Rectangle { anchors.bottom: parent.bottom; width: parent.width; height: 1; color: "#f8e6ab"; visible: row.current || (row.selected && !row.nextSelected) }
            Rectangle {
                anchors.bottom: parent.bottom
                anchors.left: parent.left
                anchors.right: parent.right
                anchors.leftMargin: 94
                anchors.rightMargin: 12
                height: 1
                color: row.selected ? "#f8e6ab" : root.app.theme.border
                visible: row.index + 1 < root.rows.length && row.selected === row.nextSelected && (row.selected || (!row.current && !row.nextCurrent))
            }

            HoverHandler {
                id: hover
            }

            TapHandler {
                acceptedButtons: Qt.LeftButton | Qt.RightButton
                onTapped: function (eventPoint, button) {
                    root.app.chooseJob(row.modelData, (point.modifiers & Qt.ControlModifier) !== 0);
                    if (button === Qt.RightButton)
                        contextMenu.popup(row, eventPoint.position.x, eventPoint.position.y);
                }
                onDoubleTapped: root.openCompleted(row.modelData)
            }

            RowLayout {
                anchors.fill: parent
                anchors.leftMargin: 12
                anchors.rightMargin: 20
                spacing: 12

                Item {
                    Layout.preferredWidth: 70
                    Layout.preferredHeight: 40
                    Rectangle {
                        id: thumbnail
                        anchors.centerIn: parent
                        width: row.modelData.kind === "audio" ? 40 : 70
                        height: 40
                        color: root.app.theme.placeholderBase
                        radius: 2
                        Image {
                            id: thumbnailImage
                            objectName: "downloadThumbnail"
                            anchors.fill: parent
                            fillMode: Image.PreserveAspectCrop
                            asynchronous: true
                            source: {
                                var image = row.modelData.thumbnail || "";
                                return image.indexOf("file:") === 0 || image.indexOf("data:") === 0 ? image : "";
                            }
                        }
                        Rectangle {
                            anchors.fill: parent
                            radius: 2
                            color: row.modelData.state === "error" ? root.app.theme.error : "black"
                            opacity: row.modelData.state === "error" ? (thumbnailImage.status === Image.Ready ? 0.4 : 0) : row.busy || row.modelData.state === "paused" ? 0.6 : row.modelData.state === "complete" ? 0.2 : 0
                        }
                        Canvas {
                            anchors.centerIn: parent
                            width: 40
                            height: 40
                            visible: row.busy || (row.modelData.state === "paused" && completeness > 0)
                            property real completeness: Math.max(0, Math.min(100, Number(row.modelData.progress || 0)))
                            property bool paused: row.modelData.state === "paused"
                            onCompletenessChanged: requestPaint()
                            onPausedChanged: requestPaint()
                            onVisibleChanged: requestPaint()
                            onPaint: {
                                var context = getContext("2d");
                                context.clearRect(0, 0, width, height);
                                context.lineWidth = 2;
                                if (!paused) {
                                    context.beginPath();
                                    context.strokeStyle = root.app.theme.secondary;
                                    context.arc(20, 20, 17, 0, Math.PI * 2);
                                    context.stroke();
                                }
                                context.beginPath();
                                context.strokeStyle = paused ? root.app.theme.secondary : "white";
                                context.arc(20, 20, 17, -Math.PI / 2, -Math.PI / 2 + completeness / 100 * Math.PI * 2);
                                context.stroke();
                            }
                        }
                        ToolButton {
                            objectName: "thumbnailAction"
                            HoverHandler { cursorShape: Qt.PointingHandCursor }
                            anchors.centerIn: parent
                            width: 24
                            height: 24
                            visible: row.busy || row.modelData.state === "paused" || (row.modelData.state === "complete" && !!row.modelData.path)
                            text: row.busy ? qsTr("Stop Download") : row.modelData.state === "paused" ? qsTr("Resume Download") : qsTr("Play")
                            Accessible.name: (row.modelData.title || "") + ". " + text
                            display: AbstractButton.IconOnly
                            icon.source: row.busy ? "" : row.modelData.state === "paused" ? "icons/download.svg" : "icons/play.svg"
                            icon.width: 20
                            icon.height: 20
                            icon.color: "white"
                            background: Rectangle { color: parent.hovered ? root.app.theme.secondary : "transparent"; radius: 2 }
                            Rectangle { anchors.centerIn: parent; width: 8; height: 8; color: "white"; visible: row.busy }
                            onClicked: {
                                root.app.chooseJob(row.modelData, false);
                                if (row.busy)
                                    root.app.jobs("pause", [row.modelData.id]);
                                else if (row.modelData.state === "paused")
                                    root.app.jobs("resume", [row.modelData.id]);
                                else
                                    root.openCompleted(row.modelData);
                            }
                        }
                        Label {
                            anchors.centerIn: parent
                            text: "!"
                            font.pixelSize: 22
                            font.bold: true
                            color: root.app.theme.error
                            visible: row.modelData.state === "error"
                        }
                        BusyIndicator {
                            anchors.centerIn: parent
                            width: 24
                            height: 24
                            running: row.modelData.state === "parsing"
                            visible: running
                        }
                    }
                }

                ColumnLayout {
                    Layout.fillWidth: true
                    spacing: 0
                    RowLayout {
                        Layout.fillWidth: true
                        spacing: 6
                        Image {
                            objectName: "downloadPlatformIcon"
                            source: root.platformIcon(row.modelData.url)
                            visible: source.toString().length > 0
                            Layout.preferredWidth: 16
                            Layout.preferredHeight: 16
                            fillMode: Image.PreserveAspectFit
                        }
                        Label {
                            textFormat: Text.PlainText
                            text: row.modelData.title || row.modelData.url || qsTr("Untitled")
                            Layout.fillWidth: true
                            elide: Text.ElideRight
                            font.pixelSize: 14
                        }
                    }
                    Label {
                        objectName: "downloadDescription"
                        textFormat: Text.PlainText
                        text: root.description(row.modelData, hover.hovered)
                        color: row.modelData.error ? root.app.theme.error : root.app.theme.secondary
                        Layout.fillWidth: true
                        elide: Text.ElideRight
                        font.pixelSize: 14
                    }
                }

                ToolButton {
                    text: qsTr("Show in Folder")
                    // Keep action geometry stable while hover changes, before the next layout polish.
                    opacity: !!row.modelData.path && (hover.hovered || row.current) ? 1 : 0
                    enabled: opacity > 0
                    Accessible.ignored: !enabled
                    display: AbstractButton.IconOnly
                    icon.source: "icons/folder.svg"
                    Accessible.name: text
                    onClicked: root.app.jobs("reveal", [row.modelData.id])
                }
                ToolButton {
                    id: moreButton
                    text: qsTr("More actions")
                    opacity: hover.hovered || row.current ? 1 : 0
                    enabled: opacity > 0
                    Accessible.ignored: !enabled
                    display: AbstractButton.IconOnly
                    palette.button: "transparent"
                    icon.source: "icons/more.svg"
                    Accessible.name: qsTr("More actions")
                    onClicked: {
                        root.app.chooseJob(row.modelData, false);
                        contextMenu.popup(moreButton, 0, moreButton.height);
                    }
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

        Label {
            textFormat: Text.PlainText
            text: root.app.search ? qsTr("No media found") : root.app.activeTab === "Subscriptions" ? qsTr("Subscribe to channels and playlists") : qsTr("Download your first video")
            font.pixelSize: 24
            color: root.app.theme.secondary
            Layout.alignment: Qt.AlignHCenter
        }

        Label {
            textFormat: Text.PlainText
            text: root.app.activeTab === "Subscriptions" ? qsTr("Paste a channel or playlist link and choose Subscribe to automatically download new videos.") : qsTr("Copy a video or song link, then click Paste Link. You can also find videos in the built-in browser.")
            wrapMode: Text.Wrap
            horizontalAlignment: Text.AlignHCenter
            Layout.fillWidth: true
            color: root.app.theme.secondary
            lineHeight: 1.4
        }
    }

    component ContextAction: MenuItem {
        leftPadding: 12
        rightPadding: 12
        topPadding: 6
        bottomPadding: 6
        implicitHeight: Math.max(32, implicitContentHeight + topPadding + bottomPadding)
    }

    component ContextSeparator: MenuSeparator {
        topPadding: 4
        bottomPadding: 4
    }

    Menu {
        id: contextMenu
        objectName: "downloadContextMenu"

        palette.window: root.app.theme.popup
        topPadding: 4
        bottomPadding: 4
        implicitWidth: {
            var widest = 240;
            for (var i = 0; i < count; ++i) {
                var item = itemAt(i);
                if (item && item.visible)
                    widest = Math.max(widest, item.implicitWidth);
            }
            return Math.ceil(widest) + leftPadding + rightPadding;
        }

        ContextAction {
            text: qsTr("Show Details")
            onTriggered: root.app.viewDetails(root.app.focusedJob)
        }

        ContextSeparator {}

        ContextAction {
            text: qsTr("Resume")
            onTriggered: root.app.jobs("resume")
        }

        ContextAction {
            text: qsTr("Pause")
            onTriggered: root.app.jobs("pause")
        }

        ContextAction {
            text: qsTr("Move to Front of Queue")
            onTriggered: root.app.jobs("force")
        }

        ContextAction {
            text: qsTr("Redownload")
            onTriggered: root.app.jobs("redownload")
        }

        ContextAction {
            text: qsTr("Show in Folder")
            onTriggered: root.app.jobs("reveal")
        }

        ContextAction {
            text: qsTr("Open File")
            onTriggered: root.app.jobs("open")
        }

        ContextSeparator {}

        ContextAction {
            objectName: "copyLinkAction"
            text: qsTr("Copy Link Address")
            onTriggered: root.app.jobs("copy_url")
        }

        ContextAction {
            text: qsTr("Open Links in Browser")
            onTriggered: root.app.jobs("open_url")
        }

        ContextAction {
            text: qsTr("Edit Subscription Settings")
            visible: root.app.activeTab === "Subscriptions"
            height: visible ? implicitHeight : 0
            onTriggered: root.app.send({
                "action": "subscriptions",
                "command": "edit",
                "ids": root.app.selectedIds
            })
        }

        ContextAction {
            text: qsTr("Unsubscribe")
            visible: root.app.activeTab === "Subscriptions"
            height: visible ? implicitHeight : 0
            onTriggered: root.app.send({
                "action": "subscriptions",
                "command": "unsubscribe",
                "ids": root.app.selectedIds
            })
        }

        ContextSeparator {}

        ContextAction {
            objectName: "deleteAction"
            text: qsTr("Delete…")
            onTriggered: root.app.confirmRemove()
        }
    }
}
