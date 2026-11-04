pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Controls
import QtQuick.Dialogs
import QtQuick.Layouts
import QtCore

AppDialog {
    id: root

    property string mode: "import"
    property string errorMessage: ""
    property url outputFolderUrl: StandardPaths.writableLocation(StandardPaths.MoviesLocation)
    readonly property bool importOldFormat: filePath.text.length > 0 && filePath.text.split(".").pop() !== "json"

    function openMode(value) {
        if (["import", "export", "subscriptions"].indexOf(value) < 0)
            return;
        mode = value;
        errorMessage = "";
        if (value === "import") {
            filePath.text = "";
            preferences.currentIndex = 0;
            completePlaylist.checked = false;
            outputFolderUrl = StandardPaths.writableLocation(StandardPaths.MoviesLocation);
            output.text = app.nativePath(outputFolderUrl);
            open();
        } else {
            close();
            Qt.callLater(function () {
                exportFile.open();
            });
        }
    }
    function submitImport() {
        const result = app.send({
            action: "import",
            path: filePath.text,
            preferences_from_file: preferences.currentIndex === 1,
            download_complete_playlist: completePlaylist.checked,
            output_dir: output.text
        });
        if (result.error)
            errorMessage = String(result.error);
        else
            close();
    }
    function exportTo(path) {
        const result = app.send({
            action: "export",
            path: path,
            subscriptions_only: mode === "subscriptions"
        });
        if (result.error) {
            errorMessage = String(result.error);
            exportError.open();
        }
    }
    function useDroppedFile(url) {
        if (String(url).indexOf("file:") !== 0)
            return false;
        filePath.text = app.nativePath(url);
        return true;
    }

    title: mode === "import" ? qsTr("Import Downloads") : mode === "subscriptions" ? qsTr("Export Subscriptions") : qsTr("Export Downloads")
    objectName: "transferDialog"
    modal: true
    anchors.centerIn: parent
    width: Math.min(490, parent.width - 40)
    height: Math.min(importContent.implicitHeight + 90, app.height - 90)
    padding: 0
    header: null
    footer: null
    closePolicy: Popup.CloseOnEscape
    Overlay.modal: Rectangle {
        color: Qt.rgba(0, 0, 0, 0.6)
    }
    background: Rectangle {
        color: root.app.theme.window
        radius: 4
    }
    onOpened: {
        if (!output.text)
            output.text = app.nativePath(outputFolderUrl);
    }

    FileDialog {
        id: importFile
        objectName: "importFileDialog"
        parentWindow: root.app
        title: qsTr("Import from")
        fileMode: FileDialog.OpenFile
        currentFolder: StandardPaths.writableLocation(StandardPaths.DocumentsLocation)
        nameFilters: ["Export Files (*.json *.csv *.4kv)", "Text Files (*.txt)", "All Files (*.*)"]
        onAccepted: filePath.text = root.app.nativePath(selectedFile)
    }
    FileDialog {
        id: exportFile
        objectName: "exportFileDialog"
        parentWindow: root.app
        title: root.title
        fileMode: FileDialog.SaveFile
        currentFolder: StandardPaths.writableLocation(StandardPaths.DocumentsLocation)
        nameFilters: ["Export Files (*.json)", "All Files (*.*)"]
        defaultSuffix: "json"
        onAccepted: root.exportTo(root.app.nativePath(selectedFile))
    }
    FolderDialog {
        id: outputFolder
        objectName: "transferFolderDialog"
        parentWindow: root.app
        title: qsTr("Save To")
        currentFolder: root.outputFolderUrl
        onAccepted: {
            root.outputFolderUrl = selectedFolder;
            output.text = root.app.nativePath(selectedFolder);
        }
    }
    AppDialog {
        id: exportError
        objectName: "transferExportErrorDialog"
        app: root.app
        parent: root.app.contentItem
        title: root.title
        modal: true
        anchors.centerIn: parent
        width: Math.min(490, parent.width - 40)
        standardButtons: Dialog.Ok
        contentItem: Label {
            text: root.errorMessage
            textFormat: Text.PlainText
            wrapMode: Text.Wrap
        }
    }

    component PathField: ColumnLayout {
        property alias label: caption.text
        property alias text: field.text
        property alias readOnly: field.readOnly
        property string fieldName: ""
        signal browseRequested
        Layout.fillWidth: true
        spacing: 8
        Label {
            id: caption
            Layout.fillWidth: true
            textFormat: Text.PlainText
            font.pixelSize: 14
        }
        RowLayout {
            Layout.fillWidth: true
            Layout.preferredHeight: 40
            spacing: 8
            TextField {
                id: field
                objectName: fieldName
                Layout.fillWidth: true
                Layout.fillHeight: true
                font.pixelSize: 14
                selectByMouse: true
                Accessible.name: caption.text
            }
            Button {
                objectName: fieldName + "Browse"
                text: qsTr("Browse")
                Layout.minimumWidth: 144
                Layout.fillHeight: true
                highlighted: true
                palette.dark: root.app.green
                onClicked: browseRequested()
            }
        }
    }

    contentItem: Item {
        ColumnLayout {
            anchors.fill: parent
            spacing: 0
            Item {
                Layout.fillWidth: true
                Layout.preferredHeight: 60
                Label {
                    x: 24
                    anchors.verticalCenter: parent.verticalCenter
                    width: parent.width - 80
                    text: root.title
                    font.pixelSize: 18
                    font.letterSpacing: 0.7
                    elide: Text.ElideRight
                }
                ToolButton {
                    id: closeButton
                    objectName: "transferCloseButton"
                    anchors.right: parent.right
                    anchors.rightMargin: 20
                    anchors.verticalCenter: parent.verticalCenter
                    width: 28
                    height: 28
                    display: AbstractButton.IconOnly
                    icon.source: root.app.asset("cross.png", "icons/close.svg")
                    icon.width: 20
                    icon.height: 20
                    icon.color: root.app.theme.text
                    Accessible.name: qsTr("Close")
                    background: Rectangle {
                        radius: 14
                        color: closeButton.hovered || closeButton.visualFocus ? root.app.theme.hover : "transparent"
                    }
                    onClicked: root.close()
                }
            }
            ScrollView {
                id: scroll
                Layout.fillWidth: true
                Layout.fillHeight: true
                Layout.bottomMargin: 24
                contentWidth: availableWidth
                contentHeight: importContent.implicitHeight
                clip: true
                ColumnLayout {
                    id: importContent
                    x: 24
                    width: scroll.availableWidth - 48
                    spacing: 24
                    Label {
                        text: root.errorMessage
                        textFormat: Text.PlainText
                        visible: text.length > 0
                        color: root.app.theme.error
                        wrapMode: Text.Wrap
                        Layout.fillWidth: true
                    }
                    PathField {
                        id: filePath
                        fieldName: "transferPath"
                        label: qsTr("Upload File")
                        onBrowseRequested: importFile.open()
                    }
                    Rectangle {
                        Layout.fillWidth: true
                        implicitHeight: 1
                        color: root.app.theme.border
                    }
                    ColumnLayout {
                        Layout.fillWidth: true
                        spacing: 8
                        Label {
                            text: qsTr("Use download settings from")
                            font.pixelSize: 14
                        }
                        ComboBox {
                            id: preferences
                            objectName: "transferPreferences"
                            Layout.fillWidth: true
                            implicitHeight: 40
                            model: [qsTr("Smart Mode Preferences"), qsTr("File Parameters")]
                            background: Rectangle {
                                color: root.app.theme.input
                                border.color: root.app.theme.border
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
                        }
                        Label {
                            Layout.fillWidth: true
                            Layout.maximumWidth: 420
                            text: qsTr("Imported files can be downloaded with the format and directory preferences specified either in the file or Smart Mode settings.")
                            textFormat: Text.PlainText
                            font.pixelSize: 14
                            lineHeight: 20
                            lineHeightMode: Text.FixedHeight
                            color: root.app.theme.secondary
                            wrapMode: Text.Wrap
                        }
                    }
                    ColumnLayout {
                        visible: root.importOldFormat
                        Layout.fillWidth: true
                        spacing: 8
                        RowLayout {
                            Layout.fillWidth: true
                            spacing: 8
                            Label {
                                Layout.fillWidth: true
                                text: qsTr("Download playlist links as series of videos")
                                font.pixelSize: 14
                                wrapMode: Text.Wrap
                            }
                            Switch {
                                id: completePlaylist
                                objectName: "transferCompletePlaylist"
                                checked: false
                                padding: 0
                                implicitWidth: 32
                                implicitHeight: 16
                                indicator: Rectangle {
                                    width: 32
                                    height: 16
                                    radius: 8
                                    color: completePlaylist.checked ? root.app.green : root.app.theme.dark ? "#666666" : "#d6d6d6"
                                    Rectangle {
                                        x: completePlaylist.checked ? 18 : 2
                                        y: 2
                                        width: 12
                                        height: 12
                                        radius: 6
                                        color: "white"
                                    }
                                }
                                Accessible.name: qsTr("Download playlist links as series of videos")
                            }
                        }
                        Label {
                            Layout.fillWidth: true
                            Layout.maximumWidth: 420
                            text: qsTr("Turn this setting on to download all contents from playlist links. Turn the setting off to only download the single videos from playlists.")
                            font.pixelSize: 14
                            lineHeight: 20
                            lineHeightMode: Text.FixedHeight
                            color: root.app.theme.secondary
                            wrapMode: Text.Wrap
                        }
                    }
                    PathField {
                        id: output
                        fieldName: "transferOutputDir"
                        label: qsTr("Output Folder")
                        readOnly: true
                        onBrowseRequested: outputFolder.open()
                    }
                    RowLayout {
                        Layout.fillWidth: true
                        layoutDirection: ["macos", "osx"].indexOf(Qt.platform.os) >= 0 ? Qt.RightToLeft : Qt.LeftToRight
                        spacing: 16
                        Button {
                            objectName: "transferImportButton"
                            text: qsTr("Import")
                            Layout.fillWidth: true
                            implicitHeight: 40
                            enabled: filePath.text.length > 0
                            highlighted: true
                            palette.dark: root.app.green
                            onClicked: root.submitImport()
                        }
                        Button {
                            objectName: "transferCancelButton"
                            text: qsTr("Cancel")
                            Layout.fillWidth: true
                            implicitHeight: 40
                            onClicked: root.close()
                        }
                    }
                }
            }
        }
        DropArea {
            anchors.fill: parent
            onDropped: drop => {
                if (drop.urls.length > 0 && root.useDroppedFile(drop.urls[0]))
                    drop.acceptProposedAction();
            }
        }
    }
}
