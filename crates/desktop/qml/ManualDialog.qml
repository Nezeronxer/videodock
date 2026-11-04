pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Controls
import QtQuick.Dialogs
import QtQuick.Layouts

AppDialog {
    id: root

    property var manual: app.state.manual || ({})
    property var selectedItems: []
    property var selectedSubtitles: []
    property var selectedAudio: []
    property bool userSelection: false
    property string selectedFormatId: ""
    property bool subscribe: false
    property bool existing: false
    property string itemSearch: ""
    property bool collapsed: false
    property bool previewReady: false
    property bool vrTouched: false
    property string outputDirectory: ""
    property string outputFilename: ""
    property bool batch: ["playlist", "channel", "search"].indexOf(manual.kind) >= 0
    property var formats: (manual.formats || []).filter(function (f) {
        return (!mediaType.currentText || !f.media_type || f.media_type === mediaType.currentText || (mediaType.currentText === "audio" && !!f.audio_codec)) && (codec.currentText === "Any" || String(f.video_codec || f.audio_codec || "") === codec.currentText) && (fps.currentText === "Any" || String(f.fps || "") === fps.currentText) && root.matchesVR(f);
    })

    function matchesVR(format) {
        var requested = vrTouched ? vr.currentText : app.settings.videoVR;
        if (requested === true)
            return format.vr === true || (typeof format.vr === "string" && ["None", "Any", "2D"].indexOf(format.vr) < 0);
        return requested === undefined || requested === false || requested === "Any" || String(format.vr) === requested;
    }

    readonly property var selectedFormat: formats.find(function (f) {
        return f.id === root.selectedFormatId;
    }) || formats[0] || ({})

    component FilterChoice: ComboBox {
        id: choice
        property string emptyLabel: root.app.optionLabel("Any")
        displayText: currentText === "Any" ? emptyLabel : root.app.optionLabel(currentText)
        leftPadding: 8
        rightPadding: 32
        background: Rectangle {
            color: choice.hovered || choice.popup.visible ? root.app.theme.hover : root.app.theme.window
            radius: 2
            border.color: root.app.theme.border
            border.width: choice.hovered || choice.popup.visible ? 0 : 1
        }
        indicator: ToolButton {
            x: choice.width - width - 8
            y: (choice.height - height) / 2
            width: 16
            height: 16
            rotation: choice.popup.visible ? -90 : 90
            enabled: false
            display: AbstractButton.IconOnly
            icon.source: "icons/forward.svg"
            icon.color: root.app.theme.text
            icon.width: 12
            icon.height: 12
            background: Item {}
        }
        contentItem: Label {
            text: choice.displayText
            color: choice.enabled ? root.app.theme.text : root.app.theme.disabled
            font.pixelSize: 14
            verticalAlignment: Text.AlignVCenter
            elide: Text.ElideRight
        }
        delegate: ItemDelegate {
            required property int index
            required property string modelData
            width: choice.width
            text: root.app.optionLabel(modelData)
            highlighted: index === choice.highlightedIndex
        }
    }

    function choices(key) {
        var values = ["Any"];
        (manual.formats || []).forEach(function (f) {
            var value = String(f[key] || "");
            if (value && !(key === "vr" && value === "Unknown") && values.indexOf(value) < 0)
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

    function downloadAction() {
        var chosen = selectedFormat;
        var action = {
            "action": "download",
            "url": manual.url,
            "kind": manual.kind,
            "mode": subscribe ? "subscribe" : "download",
            "format_id": chosen.id || "",
            "format": {
                "media_type": mediaType.currentText,
                "container": container.currentText,
                "quality": chosen.quality || "Any",
                "video_codec": chosen.video_codec || codec.currentText,
                "audio_codec": chosen.audio_codec || "Any",
                "fps": fps.currentText
            },
            "selected_ids": selectedItems,
            "subtitles": selectedSubtitles,
            "audio_tracks": selectedAudio,
            "output_dir": outputDirectory,
            "filename": outputFilename,
            "download_existing": existing
        };
        if (vrTouched)
            action.format.vr = vr.currentText;
        return action;
    }
    function refreshPreview() {
        if (!previewReady || !visible || batch || outputPath.edited || manual.state !== "ready" || !selectedFormat.id)
            return;
        var action = downloadAction();
        action.action = "preview_output";
        var result = app.send(action);
        if (result.download_path)
            outputPath.text = result.download_path;
    }
    onSelectedFormatChanged: Qt.callLater(refreshPreview)
    onClosed: previewReady = false
    function submit() {
        if (!root.batch && outputPath.edited) {
            if (!applyOutputPath(outputPath.text))
                return;
            outputPath.edited = false;
        }
        var result = app.send(downloadAction());
        if (!result.error) {
            app.manualDismissed = true;
            close();
        }
    }

    objectName: "manualDownloadDialog"
    title: subscribe ? (existing ? qsTr("Subscribe and Download") : qsTr("Subscribe")) : batch ? qsTr("Download %1").arg(manual.kind === "search" ? qsTr("Search Results") : manual.kind === "channel" ? qsTr("Channel") : qsTr("Playlist")) : mediaType.currentText === "audio" ? qsTr("Download Audio") : qsTr("Download Video")
    modal: true
    anchors.centerIn: parent
    width: Math.min(800, parent.width - 40)
    height: Math.min(batch && !collapsed ? parent.height - 40 : 520, parent.height - 40)
    padding: 0
    header: null
    footer: null
    closePolicy: Popup.CloseOnEscape
    onOpened: {
        previewReady = false;
        vrTouched = false;
        selectedFormatId = "";
        existing = manual.download_existing === true;
        selectedItems = (manual.items || []).map(function (item) {
            return item.id;
        });
        selectedSubtitles = [];
        selectedAudio = [];
        userSelection = false;
        subscribe = manual.mode === "subscribe";
        outputDirectory = manual.output_dir || app.settings.path || "";
        outputFilename = manual.filename || "";
        outputPath.text = manual.download_path || outputDirectory + (outputFilename ? "/" + outputFilename : "");
        outputPath.edited = false;
        collapsed = false;
        mediaType.currentIndex = manual.kind === "audio" || app.settings.mediaType === "audio" ? 1 : 0;
        container.currentIndex = Math.max(0, container.model.indexOf(app.optionText("container", app.settings.container)));
        vr.currentIndex = Math.max(0, vr.model.indexOf(typeof app.settings.videoVR === "string" ? app.settings.videoVR : "Any"));
        itemSearch = "";
        Qt.callLater(function () { root.previewReady = true; root.refreshPreview(); });
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
        onAccepted: {
            root.outputDirectory = root.app.nativePath(selectedFolder);
            outputPath.text = root.outputDirectory + (root.outputFilename ? "/" + root.outputFilename : "");
        }
    }

    function applyOutputPath(path) {
        var slash = path.lastIndexOf("/");
        if (slash < 0 || slash === path.length - 1) {
            root.app.localNotice = qsTr("Enter a full file path.");
            return false;
        }
        outputDirectory = path.slice(0, slash) || "/";
        outputFilename = path.slice(slash + 1);
        return true;
    }

    FileDialog {
        id: saveFile
        title: qsTr("Save To")
        fileMode: FileDialog.SaveFile
        nameFilters: mediaType.currentText === "audio" ? ["Audio (*.mp3 *.m4a *.ogg)", "All Files (*)"] : ["Video (*.mkv *.flv *.mp4 *.3gp)", "All Files (*)"]
        onAccepted: {
            var path = root.app.nativePath(selectedFile);
            if (root.applyOutputPath(path))
                outputPath.text = path;
        }
    }

    component CompactFilter: FilterChoice {
        id: filter
        implicitHeight: 20
        implicitWidth: contentItem.implicitWidth + 22
        padding: 0
        rightPadding: 22
        background: Rectangle { color: filter.hovered ? root.app.theme.hover : "transparent" }
        contentItem: Label {
            text: filter.displayText
            color: filter.enabled ? root.app.theme.text : root.app.theme.disabled
            verticalAlignment: Text.AlignVCenter
            font.pixelSize: 14
        }
    }

    contentItem: ColumnLayout {
        spacing: 0

        Rectangle {
            objectName: "manualHeader"
            Layout.fillWidth: true
            Layout.minimumHeight: root.batch ? 200 : 128
            Layout.preferredHeight: root.batch ? 238 : 128
            Layout.fillHeight: root.batch
            color: root.app.theme.window

            RowLayout {
                anchors.top: parent.top
                anchors.left: parent.left
                anchors.right: parent.right
                anchors.topMargin: 16
                anchors.leftMargin: 24
                anchors.rightMargin: 20
                height: 23
                spacing: 8
                Label {
                    text: root.title + ((root.manual.state === "parsing" || root.manual.state === "parsing_paused") ? ": " + qsTr("Retrieving information…") : "")
                    Layout.fillWidth: true
                    elide: Text.ElideRight
                    font.pixelSize: 18
                }
                ToolButton {
                    visible: root.batch
                    width: 24
                    height: 24
                    text: root.collapsed ? qsTr("Expand") : qsTr("Collapse")
                    Accessible.name: text
                    display: AbstractButton.IconOnly
                    icon.source: root.collapsed ? "icons/plus.svg" : "icons/sliders.svg"
                    onClicked: root.collapsed = !root.collapsed
                }
                ToolButton {
                    objectName: "manualCloseButton"
                    text: qsTr("Close")
                    Accessible.name: text
                    display: AbstractButton.IconOnly
                    icon.source: "icons/close.svg"
                    Layout.preferredWidth: 24
                    Layout.preferredHeight: 24
                    onClicked: root.reject()
                }
            }
            Rectangle {
                x: 24
                y: 55
                width: parent.width - 48
                height: 1
                color: root.app.theme.border
            }
            RowLayout {
                x: 24
                y: 72
                width: parent.width - 48
                height: 40
                visible: !root.batch
                spacing: 12
                Rectangle {
                    Layout.preferredWidth: mediaType.currentText === "audio" ? 40 : 70
                    Layout.preferredHeight: 40
                    color: root.app.theme.placeholderBase
                    radius: 2
                    Image {
                        anchors.fill: parent
                        fillMode: Image.PreserveAspectCrop
                        source: root.manual.thumbnail || ""
                    }
                    BusyIndicator {
                        anchors.centerIn: parent
                        width: 24
                        height: 24
                        running: root.manual.state === "parsing"
                        visible: running
                    }
                }
                ColumnLayout {
                    Layout.fillWidth: true
                    spacing: 0
                    Label {
                        textFormat: Text.PlainText
                        text: root.manual.title || root.manual.url || qsTr("Retrieving information…")
                        Layout.fillWidth: true
                        elide: Text.ElideRight
                        font.pixelSize: 14
                    }
                    Label {
                        textFormat: Text.PlainText
                        text: root.manual.error || root.manual.duration || ""
                        color: root.manual.error ? root.app.theme.error : root.app.theme.secondary
                        Layout.fillWidth: true
                        elide: Text.ElideRight
                        font.pixelSize: 14
                    }
                }
                ToolButton {
                    text: qsTr("Open Link in Browser")
                    Accessible.name: text
                    display: AbstractButton.IconOnly
                    icon.source: "icons/browser.svg"
                    enabled: !!root.manual.url
                    onClicked: Qt.openUrlExternally(root.manual.url)
                }
            }
            ColumnLayout {
                x: 24
                y: 64
                width: parent.width - 48
                height: parent.height - 72
                visible: root.batch
                spacing: 0
                RowLayout {
                    Layout.fillWidth: true
                    CheckBox {
                        text: qsTr("Select All")
                        tristate: true
                        checkState: root.selectedItems.length === 0 ? Qt.Unchecked : root.selectedItems.length === (root.manual.items || []).length ? Qt.Checked : Qt.PartiallyChecked
                        onClicked: {
                            root.userSelection = true;
                            root.selectedItems = checkState === Qt.Unchecked ? [] : (root.manual.items || []).map(function (item) { return item.id; });
                        }
                    }
                    Label {
                        text: root.selectedItems.length + qsTr(" selected")
                        color: root.app.theme.secondary
                    }
                    Item { Layout.fillWidth: true }
                    TextField {
                        placeholderText: qsTr("Search items")
                        Layout.preferredWidth: 225
                        onTextChanged: root.itemSearch = text
                    }
                    Button {
                        objectName: "parsingPauseButton"
                        visible: root.manual.state === "parsing" || root.manual.state === "parsing_paused"
                        text: root.manual.state === "parsing_paused" ? qsTr("Resume") : qsTr("Pause")
                        onClicked: root.app.send({ "action": root.manual.state === "parsing_paused" ? "manual_resume" : "manual_pause" })
                    }
                }
                ListView {
                    Layout.fillWidth: true
                    Layout.fillHeight: true
                    clip: true
                    model: (root.manual.items || []).filter(function (item) { return !root.itemSearch || (item.title || "").toLowerCase().indexOf(root.itemSearch.toLowerCase()) >= 0; })
                    ScrollBar.vertical: ScrollBar {}
                    delegate: ItemDelegate {
                        required property var modelData
                        width: ListView.view.width
                        height: root.collapsed ? 40 : 56
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
                                textFormat: Text.PlainText
                                text: modelData.title || modelData.id
                                Layout.fillWidth: true
                                elide: Text.ElideRight
                            }
                            Label { text: modelData.duration || ""; color: root.app.theme.secondary }
                            Label { text: modelData.error || modelData.size || ""; color: modelData.error ? root.app.theme.error : root.app.theme.secondary }
                        }
                    }
                }
            }
        }

        Rectangle {
            Layout.fillWidth: true
            Layout.fillHeight: !root.batch
            Layout.preferredHeight: root.batch ? 225 : -1
            Layout.minimumHeight: 200
            color: root.app.theme.hover
            ScrollView {
                anchors.fill: parent
                contentWidth: availableWidth
                clip: true
                ColumnLayout {
                    width: parent.width
                    spacing: 0
                    ColumnLayout {
                        Layout.fillWidth: true
                        Layout.leftMargin: 24
                        Layout.rightMargin: 24
                        Layout.topMargin: 16
                        spacing: 0
                        Label {
                            textFormat: Text.PlainText
                            text: root.app.localNotice || root.manual.error || ""
                            color: root.app.theme.error
                            visible: text.length > 0
                            Layout.fillWidth: true
                            wrapMode: Text.Wrap
                        }
                        RowLayout {
                            Layout.fillWidth: true
                            spacing: 16
                            ColumnLayout {
                                Layout.fillWidth: true
                                spacing: 8
                                Label { text: qsTr("Download") }
                                FilterChoice {
                                    id: mediaType
                                    model: ["video", "audio"]
                                    Layout.fillWidth: true
                                    implicitHeight: 40
                                    onActivated: { container.currentIndex = 0; codec.currentIndex = 0; }
                                }
                            }
                            ColumnLayout {
                                Layout.fillWidth: true
                                spacing: 8
                                Label { text: qsTr("Container") }
                                FilterChoice {
                                    id: container
                                    objectName: "manualContainer"
                                    model: mediaType.currentText === "audio" ? ["Any", "M4A", "MP3", "OGG", "FLAC", "WAV"] : ["Any", "MKV", "MP4"]
                                    Layout.fillWidth: true
                                    implicitHeight: 40
                                    onActivated: Qt.callLater(root.refreshPreview)
                                }
                            }
                        }
                        RowLayout {
                            Layout.fillWidth: true
                            Layout.preferredHeight: 20
                            Layout.topMargin: 24
                            spacing: 4
                            enabled: root.manual.state === "ready"
                            Label { text: qsTr("Codec"); color: root.app.theme.secondary }
                            CompactFilter {
                                id: codec
                                emptyLabel: qsTr("Auto")
                                model: root.choices("video_codec").concat(root.choices("audio_codec").slice(1)).filter(function (value, index, values) { return values.indexOf(value) === index; })
                            }
                            Label { text: qsTr("Frame Rate"); visible: fps.visible; color: root.app.theme.secondary; Layout.leftMargin: 16 }
                            CompactFilter {
                                id: fps
                                emptyLabel: qsTr("Highest")
                                model: root.choices("fps")
                                visible: mediaType.currentText !== "audio" && model.length > 1
                            }
                            Label { text: qsTr("VR"); visible: vr.visible; color: root.app.theme.secondary; Layout.leftMargin: 16 }
                            CompactFilter {
                                id: vr
                                objectName: "manualVR"
                                model: root.choices("vr")
                                displayText: !root.vrTouched && root.app.settings.videoVR === true ? "VR" : !root.vrTouched && typeof root.app.settings.videoVR === "string" ? root.app.settings.videoVR : root.app.optionLabel(currentText)
                                visible: mediaType.currentText !== "audio" && (model.length > 2 || (root.app.settings.videoVR !== undefined && root.app.settings.videoVR !== false && root.app.settings.videoVR !== "Any"))
                                onActivated: { root.vrTouched = true; Qt.callLater(root.refreshPreview); }
                            }
                            Item { Layout.fillWidth: true }
                            ToolButton {
                                id: audioLanguages
                                visible: mediaType.currentText !== "audio"
                                text: qsTr("Audio Tracks (%1)").arg(root.selectedAudio.length)
                                font.pixelSize: 14
                                padding: 0
                                implicitHeight: 20
                                background: Rectangle { color: audioLanguages.hovered ? root.app.theme.hover : "transparent" }
                                onClicked: audioPopup.open()
                                Popup {
                                    id: audioPopup
                                    width: 250
                                    height: Math.min(216, Math.max(40, ((root.manual.audio_tracks || []).length + 1) * 36 + 8))
                                    x: parent.width - width + 6
                                    y: -height - 16
                                    padding: 4
                                    background: Rectangle { color: root.app.theme.popup; border.color: root.app.theme.border }
                                    contentItem: ScrollView {
                                        contentWidth: availableWidth
                                        ColumnLayout {
                                            width: parent.width
                                            spacing: 0
                                            Label { text: (root.manual.audio_tracks || []).length ? qsTr("Choose audio tracks") : qsTr("Default audio track"); color: root.app.theme.secondary; padding: 4 }
                                            Repeater {
                                                model: root.manual.audio_tracks || []
                                                CheckBox {
                                                    required property var modelData
                                                    Layout.fillWidth: true
                                                    text: (modelData.label || modelData.language || modelData.id) + (modelData.default ? qsTr(" · default") : "")
                                                    checked: root.selectedAudio.indexOf(modelData.id) >= 0
                                                    onClicked: root.selectedAudio = root.toggle(root.selectedAudio, modelData.id)
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            ToolButton {
                                id: subtitleLanguages
                                visible: mediaType.currentText !== "audio"
                                text: qsTr("Subtitles (%1)").arg(root.selectedSubtitles.length)
                                font.pixelSize: 14
                                padding: 0
                                implicitHeight: 20
                                background: Rectangle { color: subtitleLanguages.hovered ? root.app.theme.hover : "transparent" }
                                onClicked: subtitlesPopup.open()
                                Popup {
                                    id: subtitlesPopup
                                    width: 250
                                    height: Math.min(216, Math.max(40, ((root.manual.subtitles || []).length + 1) * 36 + 8))
                                    x: parent.width - width + 6
                                    y: -height - 16
                                    padding: 4
                                    background: Rectangle { color: root.app.theme.popup; border.color: root.app.theme.border }
                                    contentItem: ScrollView {
                                        contentWidth: availableWidth
                                        ColumnLayout {
                                            width: parent.width
                                            spacing: 0
                                            Label { text: (root.manual.subtitles || []).length ? qsTr("Choose subtitle languages") : qsTr("No subtitles found for this source"); color: root.app.theme.secondary; padding: 4; wrapMode: Text.Wrap; Layout.fillWidth: true }
                                            Repeater {
                                                model: root.manual.subtitles || []
                                                CheckBox {
                                                    required property var modelData
                                                    Layout.fillWidth: true
                                                    text: (modelData.label || modelData.language || modelData.id) + (modelData.automatic ? qsTr(" · auto-generated") : "")
                                                    checked: root.selectedSubtitles.indexOf(modelData.id) >= 0
                                                    onClicked: root.selectedSubtitles = root.toggle(root.selectedSubtitles, modelData.id)
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        RowLayout {
                            Layout.fillWidth: true
                            Layout.preferredHeight: 20
                            Layout.topMargin: 8
                            spacing: 4
                            Label { text: qsTr("Quality") }
                            ToolButton {
                                text: "ⓘ"
                                implicitWidth: 20
                                implicitHeight: 20
                                ToolTip.visible: hovered
                                ToolTip.text: qsTr("In case it’s impossible to download some videos in the quality you’ve chosen, the closest quality set will be selected instead")
                            }
                            Item { Layout.fillWidth: true }
                            Label { text: qsTr("Size"); visible: !root.batch }
                            ToolButton {
                                visible: !root.batch
                                text: "ⓘ"
                                implicitWidth: 20
                                implicitHeight: 20
                                ToolTip.visible: hovered
                                ToolTip.text: qsTr("Estimated size. May vary after download.")
                            }
                        }
                        ListView {
                            id: formatOptions
                            objectName: "formatOptions"
                            Layout.fillWidth: true
                            Layout.preferredHeight: root.batch ? 0 : Math.max(152, contentHeight)
                            visible: !root.batch
                            clip: true
                            interactive: false
                            model: root.formats
                            delegate: RadioButton {
                                id: option
                                required property var modelData
                                objectName: "formatOption"
                                width: formatOptions.width
                                height: 40
                                text: [modelData.quality, modelData.video_codec || modelData.audio_codec, modelData.container].filter(Boolean).map(function (value) { return root.app.optionLabel(value); }).join(" · ")
                                checked: root.selectedFormat.id === modelData.id
                                onClicked: root.selectedFormatId = modelData.id
                                indicator: Rectangle {
                                    x: 0
                                    y: 10
                                    width: 20
                                    height: 20
                                    radius: 10
                                    color: "transparent"
                                    border.width: option.visualFocus ? 2 : 1
                                    border.color: option.checked ? root.app.green : root.app.theme.secondary
                                    Rectangle { anchors.centerIn: parent; width: 10; height: 10; radius: 5; color: root.app.green; visible: option.checked }
                                }
                                contentItem: RowLayout {
                                    spacing: 0
                                    Item {
                                        Layout.preferredWidth: 255
                                        Layout.fillHeight: true
                                        Label {
                                            x: 30
                                            anchors.verticalCenter: parent.verticalCenter
                                            width: parent.width - 34
                                            text: root.app.optionLabel(option.modelData.quality || "Original")
                                            elide: Text.ElideRight
                                        }
                                    }
                                    Label { text: option.modelData.video_codec || option.modelData.audio_codec || ""; font.pixelSize: 14 }
                                    Label { text: option.modelData.fps ? option.modelData.fps + " fps" : ""; Layout.leftMargin: 20; font.pixelSize: 14 }
                                    Item { Layout.fillWidth: true; Layout.leftMargin: 20 }
                                    Label { text: option.modelData.size ? "~ " + option.modelData.size : ""; font.pixelSize: 14 }
                                }
                                Rectangle { anchors.bottom: parent.bottom; width: parent.width; height: 1; color: root.app.theme.border }
                            }
                        }
                        ColumnLayout {
                            visible: root.batch
                            Layout.fillWidth: true
                            spacing: 9
                            Slider {
                                objectName: "formatSelector"
                                Layout.fillWidth: true
                                from: 0
                                to: Math.max(0, root.formats.length - 1)
                                stepSize: 1
                                snapMode: Slider.SnapAlways
                                enabled: root.formats.length > 0
                                value: Math.max(0, root.formats.findIndex(function (format) { return format.id === root.selectedFormatId; }))
                                onMoved: root.selectedFormatId = root.formats[Math.round(value)].id
                            }
                            RowLayout {
                                Layout.fillWidth: true
                                Repeater {
                                    model: root.formats
                                    Label { required property var modelData; text: root.app.optionLabel(modelData.quality || "Original"); Layout.fillWidth: true; horizontalAlignment: Text.AlignHCenter; color: root.app.theme.secondary }
                                }
                            }
                        }
                        ColumnLayout {
                            visible: root.formats.length === 0 && root.manual.state === "ready"
                            Layout.fillWidth: true
                            Layout.preferredHeight: 152
                            spacing: 8
                            Item { Layout.fillHeight: true }
                            Label { text: qsTr("No matching format"); color: root.app.theme.secondary; Layout.alignment: Qt.AlignHCenter }
                            Button {
                                text: qsTr("Reset Filters")
                                flat: true
                                Layout.alignment: Qt.AlignHCenter
                                onClicked: { container.currentIndex = 0; codec.currentIndex = 0; fps.currentIndex = 0; vr.currentIndex = 0; }
                            }
                            Item { Layout.fillHeight: true }
                        }
                        Item { Layout.preferredHeight: 16 }
                    }
                }
            }
        }

        Rectangle {
            objectName: "manualFooter"
            Layout.fillWidth: true
            Layout.preferredHeight: 88
            color: root.app.theme.hover
            RowLayout {
                anchors.fill: parent
                anchors.margins: 24
                spacing: 16
                TextField {
                    id: outputPath
                    objectName: "manualPath"
                    property bool edited: false
                    visible: !root.batch
                    Layout.fillWidth: true
                    Layout.preferredHeight: 40
                    rightPadding: 42
                    selectByMouse: true
                    Accessible.name: qsTr("Save To")
                    onTextEdited: edited = true
                    onEditingFinished: {
                        if (edited && root.applyOutputPath(text))
                            edited = false;
                    }
                    ToolButton {
                        text: qsTr("Browse…")
                        Accessible.name: text
                        display: AbstractButton.IconOnly
                        icon.source: "icons/folder.svg"
                        anchors.right: parent.right
                        anchors.verticalCenter: parent.verticalCenter
                        width: 36
                        height: 36
                        onClicked: saveFile.open()
                    }
                }
                ToolButton {
                    visible: root.batch
                    text: qsTr("Save To") + ": " + root.outputDirectory
                    ToolTip.visible: hovered
                    ToolTip.text: root.outputDirectory
                    onClicked: folder.open()
                    Layout.fillWidth: true
                    contentItem: Label { text: parent.text; elide: Text.ElideLeft; verticalAlignment: Text.AlignVCenter }
                }
                CheckBox {
                    text: qsTr("Subscribe to updates")
                    visible: root.batch && ["channel", "playlist"].indexOf(root.manual.kind) >= 0
                    checked: root.subscribe
                    onClicked: root.subscribe = checked
                }
                Switch {
                    text: qsTr("Download Existing")
                    visible: root.subscribe
                    checked: root.existing
                    onToggled: root.existing = checked
                }
                Button {
                    palette.dark: root.app.theme.accent
                    text: qsTr("Cancel")
                    Layout.preferredWidth: 176
                    Layout.preferredHeight: 40
                    onClicked: root.reject()
                }
                Button {
                    objectName: "downloadButton"
                    palette.dark: root.app.theme.accent
                    Layout.preferredWidth: 176
                    Layout.preferredHeight: 40
                    text: root.subscribe ? (root.existing ? qsTr("Subscribe and Download") : qsTr("Subscribe")) : root.batch ? qsTr("Download %1").arg(root.selectedItems.length) : qsTr("Download")
                    highlighted: true
                    enabled: root.manual.state === "ready" && !root.manual.error && root.formats.length > 0 && (!root.batch || root.selectedItems.length > 0 || root.subscribe)
                    onClicked: root.submit()
                }
            }
        }
    }
}
