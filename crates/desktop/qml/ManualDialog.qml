pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Controls
import QtQuick.Dialogs
import QtQuick.Layouts

Dialog {
    id: root

    required property var app
    property var manual: app.state.manual || ({})
    property var selectedItems: []
    property var selectedSubtitles: []
    property var selectedAudio: []
    property bool userSelection: false
    property string selectedFormatId: ""
    property bool subscribe: false
    property bool existing: false
    property string itemSearch: ""
    property bool batch: ["playlist", "channel", "search"].indexOf(manual.kind) >= 0
    property var formats: (manual.formats || []).filter(function (f) {
        return (!mediaType.currentText || !f.media_type || f.media_type === mediaType.currentText) && (quality.currentText === "Any" || !f.quality || String(f.quality) === quality.currentText) && (container.currentText === "Any" || !f.container || String(f.container).toUpperCase() === container.currentText) && (codec.currentText === "Any" || String(f.video_codec || f.audio_codec || "") === codec.currentText) && (fps.currentText === "Any" || String(f.fps || "") === fps.currentText) && (vr.currentText === "Any" || String(f.vr || "") === vr.currentText);
    })

    function choices(key) {
        var values = ["Any"];
        (manual.formats || []).forEach(function (f) {
            var value = String(f[key] || "");
            if (value && values.indexOf(value) < 0)
                values.push(value);
        });
        return values;
    }

    function toggle(array, value) {
        var next = array.slice();
        var index = next.indexOf(value);
        if (index < 0)
            next.push(value);
        else
            next.splice(index, 1);
        return next;
    }

    function submit() {
        var chosen = formats[formatChoice.currentIndex] || ({});
        var result = app.send({
            "action": "download",
            "url": manual.url,
            "kind": manual.kind,
            "mode": subscribe ? "subscribe" : "download",
            "format_id": chosen.id || "",
            "format": {
                "media_type": mediaType.currentText,
                "container": container.currentText,
                "quality": quality.currentText,
                "video_codec": chosen.video_codec || codec.currentText,
                "audio_codec": chosen.audio_codec || "Any",
                "fps": fps.currentText,
                "vr": vr.currentText
            },
            "selected_ids": selectedItems,
            "subtitles": selectedSubtitles,
            "audio_tracks": selectedAudio,
            "output_dir": output.text,
            "filename": filename.text,
            "download_existing": existing
        });
        if (!result.error) {
            app.manualDismissed = true;
            close();
        }
    }

    objectName: "manualDownloadDialog"
    title: subscribe ? (existing ? qsTr("Subscribe and Download") : qsTr("Subscribe")) : batch ? qsTr("Download %1").arg(manual.kind === "search" ? qsTr("Search Results") : manual.kind === "channel" ? qsTr("Channel") : qsTr("Playlist")) : qsTr("Download Video")
    modal: true
    anchors.centerIn: parent
    width: Math.min(850, parent.width - 45)
    height: Math.min(650, parent.height - 45)
    closePolicy: Popup.CloseOnEscape
    onOpened: {
        selectedFormatId = "";
        existing = manual.download_existing === true;
        selectedItems = (manual.items || []).map(function (item) {
            return item.id;
        });
        selectedSubtitles = [];
        selectedAudio = [];
        userSelection = false;
        subscribe = manual.mode === "subscribe";
        output.text = manual.output_dir || app.settings.path || "";
        filename.text = manual.filename || "";
        mediaType.currentIndex = manual.kind === "audio" || app.settings.mediaType === "audio" ? 1 : 0;
        itemSearch = "";
    }
    onManualChanged: {
        if (visible && !userSelection) {
            selectedItems = (manual.items || []).map(function (item) {
                return item.id;
            });
        }
    }
    onRejected: {
        app.manualDismissed = true;
        app.send({
            "action": "manual_close"
        });
    }

    FolderDialog {
        id: folder

        title: qsTr("Save To")
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
        spacing: 12

        Label {
 textFormat: Text.PlainText;
            text: root.manual.title || root.manual.url || qsTr("Retrieving information…")
            font.pixelSize: 19
            font.bold: true
            Layout.fillWidth: true
            elide: Text.ElideRight
        }

        Label {
 textFormat: Text.PlainText;
            text: root.manual.url || ""
            Layout.fillWidth: true
            elide: Text.ElideMiddle
            color: "#888"
            font.pixelSize: 12
        }

        BusyIndicator {
            running: root.manual.state === "parsing"
            visible: running
            Layout.alignment: Qt.AlignHCenter
        }

        RowLayout {
            visible: root.manual.state === "parsing" || root.manual.state === "parsing_paused"
            Layout.fillWidth: true
            Label {
                textFormat: Text.PlainText
                text: root.manual.total_count ? qsTr("%1 / %2").arg(root.manual.parsed_count || 0).arg(root.manual.total_count) : qsTr("Retrieving information…")
                Layout.fillWidth: true
            }
            Button {
                objectName: "parsingPauseButton"
                text: root.manual.state === "parsing_paused" ? qsTr("Resume") : qsTr("Pause")
                onClicked: root.app.send({"action": root.manual.state === "parsing_paused" ? "manual_resume" : "manual_pause"})
            }
        }

        Label {
 textFormat: Text.PlainText;
            text: root.manual.error || ""
            visible: text.length > 0
            color: "#b94332"
            wrapMode: Text.Wrap
            Layout.fillWidth: true
        }

        GridLayout {
            columns: 6
            columnSpacing: 12
            rowSpacing: 7
            Layout.fillWidth: true

            Label {
 textFormat: Text.PlainText;
                text: qsTr("Download")
            }

            Label {
 textFormat: Text.PlainText;
                text: qsTr("Quality")
            }

            Label {
 textFormat: Text.PlainText;
                text: qsTr("Container")
            }

            Label {
 textFormat: Text.PlainText;
                text: qsTr("Codec")
            }

            Label {
 textFormat: Text.PlainText;
                text: qsTr("Frame Rate")
            }

            Label {
 textFormat: Text.PlainText;
                text: qsTr("VR")
            }

            ComboBox {
                id: mediaType

                model: ["video", "audio"]
                Layout.fillWidth: true
                onActivated: {
                    quality.currentIndex = 0;
                    container.currentIndex = 0;
                    codec.currentIndex = 0;
                }
            }

            ComboBox {
                id: quality

                model: root.choices("quality")
                Layout.fillWidth: true
            }

            ComboBox {
                id: container

                model: root.choices("container")
                Layout.fillWidth: true
            }

            ComboBox {
                id: codec

                model: {
                    var values = root.choices("video_codec").concat(root.choices("audio_codec").slice(1));
                    return values.filter(function (v, i) {
                        return values.indexOf(v) === i;
                    });
                }
                Layout.fillWidth: true
            }

            ComboBox {
                id: fps

                model: root.choices("fps")
                Layout.fillWidth: true
            }

            ComboBox {
                id: vr

                model: root.choices("vr")
                Layout.fillWidth: true
            }
        }

        RowLayout {
            Layout.fillWidth: true

            Label {
 textFormat: Text.PlainText;
                text: qsTr("Available Format")
                color: "#777"
            }

            ComboBox {
                id: formatChoice

                objectName: "formatSelector"
                Layout.fillWidth: true
                model: root.formats
                currentIndex: Math.max(0, root.formats.findIndex(function (f) {
                    return f.id === root.selectedFormatId;
                }))
                onActivated: root.selectedFormatId = root.formats[currentIndex].id
                textRole: "label"
                displayText: currentIndex >= 0 && root.formats[currentIndex] ? (root.formats[currentIndex].label || [root.formats[currentIndex].container, root.formats[currentIndex].quality, root.formats[currentIndex].video_codec || root.formats[currentIndex].audio_codec].filter(Boolean).join(" · ")) : qsTr("No matching format")
            }

            Button {
                text: qsTr("Reset Filters")
                onClicked: {
                    quality.currentIndex = 0;
                    container.currentIndex = 0;
                    codec.currentIndex = 0;
                    fps.currentIndex = 0;
                    vr.currentIndex = 0;
                }
            }
        }

        TabBar {
            id: extraTabs

            Layout.fillWidth: true
            currentIndex: root.batch ? 0 : 1

            TabButton {
                text: qsTr("Items (%1)").arg((root.manual.items || []).length)
                visible: root.batch
                width: visible ? implicitWidth : 0
            }

            TabButton {
                text: qsTr("Subtitles (%1)").arg((root.manual.subtitles || []).length)
            }

            TabButton {
                text: qsTr("Audio Tracks (%1)").arg((root.manual.audio_tracks || []).length)
            }
        }

        StackLayout {
            currentIndex: extraTabs.currentIndex
            Layout.fillWidth: true
            Layout.fillHeight: true

            ColumnLayout {
                RowLayout {
                    Layout.fillWidth: true

                    CheckBox {
                        text: qsTr("Select All")
                        checked: root.selectedItems.length === (root.manual.items || []).length && root.selectedItems.length > 0
                        onClicked: {
                            root.userSelection = true;
                            root.selectedItems = checked ? (root.manual.items || []).map(function (i) {
                                return i.id;
                            }) : [];
                        }
                    }

                    Label {
 textFormat: Text.PlainText;
                        text: root.selectedItems.length + qsTr(" selected")
                        color: "#888"
                    }

                    Item {
                        Layout.fillWidth: true
                    }

                    TextField {
                        placeholderText: qsTr("Search items")
                        onTextChanged: root.itemSearch = text
                        Layout.preferredWidth: 225
                    }
                }

                ListView {
                    Layout.fillWidth: true
                    Layout.fillHeight: true
                    clip: true
                    model: (root.manual.items || []).filter(function (item) {
                        return !root.itemSearch || (item.title || "").toLowerCase().indexOf(root.itemSearch.toLowerCase()) >= 0;
                    })

                    ScrollBar.vertical: ScrollBar {}

                    delegate: ItemDelegate {
                        required property var modelData

                        width: ListView.view.width
                        height: 52
                        onClicked: {
                            root.userSelection = true;
                            root.selectedItems = root.toggle(root.selectedItems, modelData.id);
                        }

                        contentItem: RowLayout {
                            CheckBox {
                                checked: root.selectedItems.indexOf(modelData.id) >= 0
                                onClicked: {
                                    root.userSelection = true;
                                    root.selectedItems = root.toggle(root.selectedItems, modelData.id);
                                }
                            }

                            Label {
 textFormat: Text.PlainText;
                                text: modelData.title || modelData.id
                                Layout.fillWidth: true
                                elide: Text.ElideRight
                            }

                            Label {
 textFormat: Text.PlainText;
                                text: modelData.duration || ""
                                color: "#888"
                            }

                            Label {
 textFormat: Text.PlainText;
                                text: modelData.error || modelData.size || ""
                                color: modelData.error ? "#b94332" : "#888"
                            }
                        }
                    }
                }
            }

            ScrollView {
                contentWidth: availableWidth

                ColumnLayout {
                    width: parent.width

                    Label {
 textFormat: Text.PlainText;
                        text: (root.manual.subtitles || []).length ? qsTr("Choose subtitle languages") : qsTr("No subtitles found for this source")
                        color: "#888"
                        padding: 8
                    }

                    Repeater {
                        model: root.manual.subtitles || []

                        CheckBox {
                            required property var modelData

                            text: (modelData.label || modelData.language || modelData.id) + (modelData.automatic ? qsTr(" · auto-generated") : "")
                            checked: root.selectedSubtitles.indexOf(modelData.id) >= 0
                            onClicked: root.selectedSubtitles = root.toggle(root.selectedSubtitles, modelData.id)
                        }
                    }
                }
            }

            ScrollView {
                contentWidth: availableWidth

                ColumnLayout {
                    width: parent.width

                    Label {
 textFormat: Text.PlainText;
                        text: (root.manual.audio_tracks || []).length ? qsTr("Choose audio tracks") : qsTr("Default audio track")
                        color: "#888"
                        padding: 8
                    }

                    Repeater {
                        model: root.manual.audio_tracks || []

                        CheckBox {
                            required property var modelData

                            text: (modelData.label || modelData.language || modelData.id) + (modelData.default ? qsTr(" · default") : "")
                            checked: root.selectedAudio.indexOf(modelData.id) >= 0
                            onClicked: root.selectedAudio = root.toggle(root.selectedAudio, modelData.id)
                        }
                    }
                }
            }
        }

        RowLayout {
            Layout.fillWidth: true

            Label {
 textFormat: Text.PlainText;
                text: qsTr("Save To")
            }

            TextField {
                id: output

                Layout.fillWidth: true
                selectByMouse: true
            }

            Button {
                text: qsTr("Browse…")
                onClicked: folder.open()
            }
        }

        RowLayout {
            visible: !root.batch

            Label {
 textFormat: Text.PlainText;
                text: qsTr("File Name")
            }

            TextField {
                id: filename

                Layout.fillWidth: true
                selectByMouse: true
                placeholderText: qsTr("Keep source title")
            }
        }

        RowLayout {
            Layout.fillWidth: true

            CheckBox {
                text: qsTr("Subscribe to updates")
                visible: ["channel", "playlist"].indexOf(root.manual.kind) >= 0
                checked: root.subscribe
                onClicked: root.subscribe = checked
            }

            CheckBox {
                text: qsTr("Download Existing")
                visible: root.subscribe
                checked: root.existing
                onClicked: root.existing = checked
            }

            Item {
                Layout.fillWidth: true
            }

            Button {
                text: qsTr("Cancel")
                onClicked: root.reject()
            }

            Button {
                objectName: "downloadButton"
                text: root.subscribe ? (root.existing ? qsTr("Subscribe and Download") : qsTr("Subscribe")) : root.batch ? qsTr("Download %1").arg(root.selectedItems.length) : qsTr("Download")
                highlighted: true
                enabled: root.manual.state === "ready" && !root.manual.error && root.formats.length > 0 && (!root.batch || root.selectedItems.length > 0 || root.subscribe)
                onClicked: root.submit()
            }
        }
    }
}
