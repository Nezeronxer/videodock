pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Controls
import QtQuick.Dialogs
import QtQuick.Layouts

Dialog {
    id: root

    required property var app
    property string mode: "import"

    function openMode(value) {
        mode = value;
        filePath.clear();
        output.text = app.settings.path || "";
        open();
    }

    title: mode === "import" ? qsTr("Import Downloads") : mode === "subscriptions" ? qsTr("Export Subscriptions") : qsTr("Export Downloads")
    objectName: "transferDialog"
    modal: true
    anchors.centerIn: parent
    width: Math.min(620, parent.width - 50)

    FileDialog {
        id: fileDialog

        title: root.title
        fileMode: root.mode === "import" ? FileDialog.OpenFile : FileDialog.SaveFile
        nameFilters: ["JSON files (*.json)", "All files (*)"]
        defaultSuffix: "json"
        onAccepted: filePath.text = root.app.nativePath(selectedFile)
    }

    FolderDialog {
        id: outputFolder

        title: qsTr("Output Folder")
        onAccepted: output.text = root.app.nativePath(selectedFolder)
    }

    contentItem: ColumnLayout {
        Label {
 textFormat: Text.PlainText;
            text: root.app.localNotice
            visible: text.length > 0
            color: "#b94332"
            wrapMode: Text.Wrap
            Layout.fillWidth: true
        }
        spacing: 16

        Label {
 textFormat: Text.PlainText;
            text: root.mode === "import" ? qsTr("Import links, download settings and subscriptions from a JSON file.") : qsTr("Save links and download preferences as a JSON file.")
            wrapMode: Text.Wrap
            Layout.fillWidth: true
            color: "#777"
        }

        Label {
 textFormat: Text.PlainText;
            text: root.mode === "import" ? qsTr("Upload File") : qsTr("Export To")
        }

        RowLayout {
            Layout.fillWidth: true

            TextField {
                id: filePath

                objectName: "transferPath"
                Layout.fillWidth: true
                placeholderText: "/path/downloads.json"
                selectByMouse: true
            }

            Button {
                text: qsTr("Browse…")
                onClicked: fileDialog.open()
            }
        }

        ComboBox {
            id: preferences

            visible: root.mode === "import"
            Layout.fillWidth: true
            model: [qsTr("Use download settings from file"), qsTr("Use current application settings")]
        }

        CheckBox {
            id: completePlaylist

            visible: root.mode === "import"
            text: qsTr("Download complete playlists and channels")
            checked: true
        }

        RowLayout {
            visible: root.mode === "import"
            Layout.fillWidth: true

            Label {
 textFormat: Text.PlainText;
                text: qsTr("Output Folder")
            }

            TextField {
                id: output

                Layout.fillWidth: true
            }

            Button {
                text: qsTr("Browse…")
                onClicked: outputFolder.open()
            }
        }

        RowLayout {
            Item {
                Layout.fillWidth: true
            }

            Button {
                text: qsTr("Cancel")
                onClicked: root.close()
            }

            Button {
                text: root.mode === "import" ? qsTr("Import") : qsTr("Export")
                highlighted: true
                enabled: filePath.text.trim().length > 0
                onClicked: {
                    var action = root.mode === "import" ? {
                        "action": "import",
                        "path": filePath.text,
                        "preferences_from_file": preferences.currentIndex === 0,
                        "download_complete_playlist": completePlaylist.checked,
                        "output_dir": output.text
                    } : {
                        "action": "export",
                        "path": filePath.text,
                        "subscriptions_only": root.mode === "subscriptions"
                    };
                    var result = root.app.send(action);
                    if (!result.error)
                        root.close();
                }
            }
        }
    }
}
