pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

Dialog {
    id: root

    required property var app
    property var job: app.focusedJob || ({})

    title: qsTr("Details")
    objectName: "detailsDialog"
    modal: true
    anchors.centerIn: parent
    width: Math.min(650, parent.width - 50)
    height: Math.min(500, parent.height - 50)
    standardButtons: Dialog.Close

    contentItem: ColumnLayout {
        spacing: 12

        Label {
 textFormat: Text.PlainText;
            text: root.job.title || root.job.url || ""
            font.pixelSize: 20
            font.bold: true
            wrapMode: Text.Wrap
            Layout.fillWidth: true
        }

        TextField {
            text: root.job.url || ""
            readOnly: true
            selectByMouse: true
            Layout.fillWidth: true
        }

        GridLayout {
            columns: 2
            rowSpacing: 9
            columnSpacing: 18
            Layout.fillWidth: true

            Repeater {
                model: [qsTr("Type"), root.job.kind || "", qsTr("Status"), root.job.state || "", qsTr("Format"), [root.job.format, root.job.quality].filter(Boolean).join(" · "), qsTr("Size"), root.job.size || "", qsTr("Save To"), root.job.path || "", qsTr("Progress"), root.job.progress === undefined ? "" : root.job.progress + "%"]

                Label {
 textFormat: Text.PlainText;
                    required property string modelData

                    text: modelData
                    wrapMode: Text.Wrap
                    Layout.fillWidth: true
                }
            }
        }

        Label {
 textFormat: Text.PlainText;
            text: root.job.error || ""
            color: "#b94332"
            wrapMode: Text.Wrap
            Layout.fillWidth: true
        }

        Item {
            Layout.fillHeight: true
        }

        RowLayout {
            Button {
                text: qsTr("Show in Folder")
                enabled: !!root.job.path
                onClicked: root.app.jobs("reveal", [root.job.id])
            }

            Button {
                text: qsTr("Open Link")
                onClicked: root.app.jobs("open_url", [root.job.id])
            }

            Button {
                text: qsTr("Redownload")
                onClicked: root.app.jobs("redownload", [root.job.id])
            }
        }

        RowLayout {
            visible: ["playlist", "channel"].indexOf(root.job.kind) >= 0

            Button {
                text: qsTr("Subscribe")
                onClicked: {
                    root.app.parse(root.job.url);
                    root.close();
                }
            }

            Button {
                text: qsTr("Edit Settings")
                onClicked: {
                    root.app.send({
                        "action": "subscriptions",
                        "command": "edit",
                        "ids": [root.job.id]
                    });
                    root.close();
                }
            }
        }
    }
}
