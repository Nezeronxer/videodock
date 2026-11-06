pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Controls
import QtQuick.Dialogs
import QtQuick.Layouts
import QtQml.Models

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
    property var editedFormat: ({})
    property string operatingSystem: "Any"
    property bool preferencesInitialized: false
    property bool modeTouched: false
    readonly property var savedPreferences: manual.editing ? manual.preferences || {} : ({})
    readonly property var requestedVR: vrTouched ? vr.currentText : manual.editing ? (savedPreferences.vr === true ? true : savedPreferences.vr_mode || "Any") : app.settings.videoVR
    property string outputDirectory: ""
    property string outputFilename: ""
    property bool batch: ["playlist", "channel", "search"].indexOf(manual.kind) >= 0
    property var formats: (manual.formats || []).filter(function (f) {
        return (!mediaType.currentText || !f.media_type || f.media_type === mediaType.currentText || (mediaType.currentText === "audio" && !!f.audio_codec)) && (codec.currentText === "Any" || String(f.video_codec || f.audio_codec || "") === codec.currentText) && (fps.currentText === "Any" || String(f.fps || "") === fps.currentText) && root.matchesVR(f);
    })

    function matchesVR(format) {
        var requested = requestedVR;
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
        if (manual.editing) {
            var saved = manual.preferences || {};
            ["media_type", "container", "quality", "video_codec", "audio_codec", "audio_bitrate", "fps", "vr", "vr_mode"].forEach(function (key) {
                if (!root.editedFormat[key] && saved[key] !== undefined)
                    action.format[key] = saved[key];
            });
            if (!editedFormat.format_id)
                action.format_id = saved.format_id || "";
        }
        if (operatingSystem !== "Any") {
            action.format.video_codec = "Any";
            action.format.audio_codec = "Any";
            action.format_id = "";
        }
        return action;
    }
    function editFormat(keys) {
        var next = Object.assign({}, editedFormat);
        keys.forEach(function (key) { next[key] = true; });
        editedFormat = next;
    }
    function applyOperatingSystem(value) {
        operatingSystem = value;
        if (value === "Any") return;
        var audio = mediaType.currentText === "audio";
        var valueContainer = audio ? (value === "Linux" ? "OGG" : value === "Mac OS" || value === "iOS" ? "M4A" : "MP3") : value === "Linux" ? "MKV" : "MP4";
        container.currentIndex = container.model.indexOf(valueContainer);
        codec.currentIndex = 0;
        root.editFormat(["container", "video_codec", "audio_codec", "format_id"]);
    }
    function chooseFolder(path) {
        outputDirectory = path;
        outputPath.text = path + (outputFilename ? "/" + outputFilename : "");
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
    title: manual.editing ? qsTranslate("JobList", "Edit Subscription Settings") : subscribe ? (existing ? qsTr("Subscribe and Download") : qsTr("Subscribe")) : batch ? qsTr("Download %1").arg(manual.kind === "search" ? qsTr("Search Results") : manual.kind === "channel" ? qsTr("Channel") : qsTr("Playlist")) : mediaType.currentText === "audio" ? qsTr("Download Audio") : qsTr("Download Video")
    modal: true
    anchors.centerIn: parent
    width: Math.min(800, parent.width - 40)
    height: Math.min(batch && !collapsed ? parent.height - 40 : 520, parent.height - 40)
    padding: 0
    header: null
    footer: null
    closePolicy: Popup.CloseOnEscape
    onOpened: initializeSelection()
    function initializeSelection() {
        preferencesInitialized = manual.state === "ready";
        previewReady = false;
        modeTouched = false;
        vrTouched = false;
        editedFormat = {};
        operatingSystem = !manual.editing && batch ? app.settings.containerOSAlias || "Any" : "Any";
        operatingSystemChoice.currentIndex = Math.max(0, operatingSystemChoice.model.indexOf(operatingSystem));
        selectedFormatId = "";
        var saved = manual.editing ? manual.preferences || {} : {};
        existing = !manual.editing && manual.download_existing === true;
        selectedItems = (manual.items || []).map(function (item) {
            return item.id;
        });
        selectedSubtitles = (saved.subtitles || []).slice();
        selectedAudio = (saved.audio_tracks || []).slice();
        userSelection = false;
        subscribe = manual.mode === "subscribe";
        outputDirectory = manual.output_dir || app.settings.path || "";
        outputFilename = manual.filename || "";
        outputPath.text = manual.download_path || outputDirectory + (outputFilename ? "/" + outputFilename : "");
        outputPath.edited = false;
        collapsed = false;
        mediaType.currentIndex = manual.kind === "audio" || (saved.media_type || app.settings.mediaType) === "audio" ? 1 : 0;
        container.currentIndex = Math.max(0, container.model.indexOf(app.optionText("container", saved.container || app.settings.container)));
        codec.currentIndex = 0;
        fps.currentIndex = 0;
        vr.currentIndex = Math.max(0, vr.model.indexOf(saved.vr_mode || (typeof app.settings.videoVR === "string" ? app.settings.videoVR : "Any")));
        selectedFormatId = manual.selected_format_id || saved.format_id || "";
        if (manual.editing) {
            var chosen = selectedFormat;
            codec.currentIndex = Math.max(0, codec.model.indexOf(mediaType.currentText === "audio" ? chosen.audio_codec : chosen.video_codec));
            fps.currentIndex = Math.max(0, fps.model.indexOf(String(saved.fps || "Any")));
        }
        if (operatingSystem !== "Any") applyOperatingSystem(operatingSystem);
        itemSearch = "";
        Qt.callLater(function () { root.previewReady = true; root.refreshPreview(); });
    }
    onManualChanged: {
        if (visible && manual.state === "ready" && !preferencesInitialized) {
            if (manual.editing)
                initializeSelection();
            else {
                if (!modeTouched) subscribe = manual.mode === "subscribe";
                preferencesInitialized = true;
                if (manual.kind === "audio" && !editedFormat.media_type)
                    mediaType.currentIndex = 1;
                if (operatingSystem !== "Any") applyOperatingSystem(operatingSystem);
            }
        }
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
            root.chooseFolder(root.app.nativePath(selectedFolder));
        }
    }
    Menu {
        id: outputHistory
        objectName: "manualFolderHistory"
        Instantiator {
            model: root.app.settings.paths || []
            delegate: MenuItem {
                required property string modelData
                text: modelData
                onTriggered: root.chooseFolder(modelData)
            }
            onObjectAdded: (index, object) => outputHistory.insertItem(index, object)
            onObjectRemoved: (index, object) => outputHistory.removeItem(object)
        }
        MenuSeparator {}
        MenuItem { text: qsTr("Browse…"); onTriggered: folder.open() }
    }

    function applyOutputPath(path) {
        if (Qt.platform.os === "windows")
            path = path.replace(/\\/g, "/");
        var slash = path.lastIndexOf("/");
        if (slash < 0 || slash === path.length - 1) {
            root.app.localNotice = qsTr("Enter a full file path.");
            return false;
        }
        outputDirectory = path.slice(0, slash) || "/";
        if (Qt.platform.os === "windows" && /^[A-Za-z]:$/.test(outputDirectory))
            outputDirectory += "/";
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
                                    objectName: "manualMediaType"
                                    model: ["video", "audio"]
                                    Layout.fillWidth: true
                                    implicitHeight: 40
                                    onActivated: {
                                        container.currentIndex = 0; codec.currentIndex = 0;
                                        root.editFormat(["media_type", "container", "quality", "video_codec", "audio_codec", "audio_bitrate", "fps", "vr", "vr_mode", "format_id"]);
                                        if (root.operatingSystem !== "Any") root.applyOperatingSystem(root.operatingSystem);
                                    }
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
                                    onActivated: { root.operatingSystem = "Any"; operatingSystemChoice.currentIndex = 0; root.editFormat(["container"]); Qt.callLater(root.refreshPreview); }
                                }
                            }
                            ColumnLayout {
                                visible: root.batch
                                Layout.fillWidth: true
                                spacing: 8
                                Label { text: qsTranslate("SmartModeBar", "For") }
                                FilterChoice {
                                    id: operatingSystemChoice
                                    objectName: "manualOperatingSystem"
                                    model: ["Any", "Windows", "Mac OS", "Linux", "iOS", "Android"]
                                    Layout.fillWidth: true
                                    implicitHeight: 40
                                    onActivated: root.applyOperatingSystem(currentText)
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
                                objectName: "manualCodec"
                                emptyLabel: qsTr("Auto")
                                model: root.choices("video_codec").concat(root.choices("audio_codec").slice(1)).filter(function (value, index, values) { return values.indexOf(value) === index; })
                                onActivated: root.editFormat([mediaType.currentText === "audio" ? "audio_codec" : "video_codec", "format_id"])
                            }
                            Label { text: qsTr("Frame Rate"); visible: fps.visible; color: root.app.theme.secondary; Layout.leftMargin: 16 }
                            CompactFilter {
                                id: fps
                                emptyLabel: qsTr("Highest")
                                model: root.choices("fps")
                                visible: mediaType.currentText !== "audio" && model.length > 1
                                onActivated: root.editFormat(["fps", "format_id"])
                            }
                            Label { text: qsTr("VR"); visible: vr.visible; color: root.app.theme.secondary; Layout.leftMargin: 16 }
                            CompactFilter {
                                id: vr
                                objectName: "manualVR"
                                model: root.choices("vr")
                                displayText: root.requestedVR === true ? "VR" : typeof root.requestedVR === "string" ? root.app.optionLabel(root.requestedVR) : root.app.optionLabel(currentText)
                                visible: mediaType.currentText !== "audio" && (model.length > 2 || (root.requestedVR !== undefined && root.requestedVR !== false && root.requestedVR !== "Any"))
                                onActivated: { root.vrTouched = true; root.editFormat(["vr", "vr_mode", "format_id"]); Qt.callLater(root.refreshPreview); }
                            }
                            Item { Layout.fillWidth: true }
                            ToolButton {
                                id: audioLanguages
                                objectName: "manualAudioTracks"
                                visible: mediaType.currentText !== "audio" || (root.manual.audio_tracks || []).length > 0
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
                                onClicked: { root.selectedFormatId = modelData.id; root.editFormat(["quality", "audio_bitrate", "video_codec", "audio_codec", "fps", "format_id"]); }
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
                                onMoved: { root.selectedFormatId = root.formats[Math.round(value)].id; root.editFormat(["quality", "audio_bitrate", "video_codec", "audio_codec", "fps", "format_id"]); }
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
                                onClicked: { container.currentIndex = 0; codec.currentIndex = 0; fps.currentIndex = 0; vr.currentIndex = 0; root.vrTouched = true; root.operatingSystem = "Any"; operatingSystemChoice.currentIndex = 0; root.editFormat(["container", "video_codec", "audio_codec", "fps", "vr", "vr_mode", "format_id"]); }
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
                    id: outputFolder
                    objectName: "manualOutputFolder"
                    text: qsTr("Save To") + ": " + root.outputDirectory
                    ToolTip.visible: hovered
                    ToolTip.text: root.outputDirectory
                    onClicked: (root.app.settings.paths || []).length ? outputHistory.popup(outputFolder, 0, outputFolder.height) : folder.open()
                    Layout.fillWidth: true
                    contentItem: Label { text: parent.text; elide: Text.ElideLeft; verticalAlignment: Text.AlignVCenter }
                }
                CheckBox {
                    text: qsTr("Subscribe to updates")
                    visible: !root.manual.editing && root.batch && ["channel", "playlist"].indexOf(root.manual.kind) >= 0
                    checked: root.subscribe
                    onClicked: { root.subscribe = checked; root.modeTouched = true; }
                }
                Switch {
                    objectName: "manualDownloadExisting"
                    text: qsTr("Download Existing")
                    visible: root.subscribe && !root.manual.editing
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
                    text: root.manual.editing ? qsTranslate("SettingsDialog", "Save") : root.subscribe ? (root.existing ? qsTr("Subscribe and Download") : qsTr("Subscribe")) : root.batch ? qsTr("Download %1").arg(root.selectedItems.length) : qsTr("Download")
                    highlighted: true
                    enabled: root.manual.state === "ready" && !root.manual.error && (root.formats.length > 0 || (root.manual.editing && Object.keys(root.editedFormat).length === 0)) && (!root.batch || root.selectedItems.length > 0 || root.subscribe)
                    onClicked: root.submit()
                }
            }
        }
    }
}
