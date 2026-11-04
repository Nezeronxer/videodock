pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Controls
import QtQuick.Controls.impl
import QtQuick.Layouts
import QtQml.Models
import "LanguageOptions.js" as Languages
import Qt5Compat.GraphicalEffects

RowLayout {
    id: root
    required property var app
    signal browseRequested()
    readonly property bool audio: app.settings.mediaType === "audio"
    spacing: 16

    function rawValue(key, value) {
        if (key === "audioBitrate")
            return Number(value) === 0 ? "Original" : value + "kbps";
        if (key === "frameRate" && Number(value) === 0)
            return "Maximum";
        if (key === "videoVR")
            return typeof value === "string" ? value : value === true ? "VR" : "Any";
        return root.app.optionText(key, value);
    }

    function folderName(path) {
        var parts = String(path || "").split(/[\\/]/).filter(Boolean);
        if (parts[parts.length - 1] === "VideoDock")
            parts.pop();
        return parts.pop() || String(path || "");
    }
    function label(key, value) {
        if (key === "subtitles" || key === "audioTracks") {
            if (value === "-")
                return key === "subtitles" ? qsTr("None") : qsTr("Default");
            if (value === "default")
                return qsTr("Default");
            if (value === "all")
                return qsTr("All Tracks");
            if (value === "original")
                return qsTr("Original");
            var option = (key === "subtitles" ? Languages.subtitles : Languages.audioTracks).find(function (item) { return item.language === value; });
            return option ? option.title : value;
        }
        if (key === "audioBitrate")
            return value === "Original" || Number(value) === 0 ? qsTr("Highest") : value;
        if (key === "quality")
            return ({"8K":"UHD 8K", "4K":"HD 4K", "1440p":"HQ 2K"})[value] || root.app.optionLabel(value);
        if (key === "frameRate")
            return value === "Maximum" ? qsTr("Highest") : value ? value + "fps" : "";
        if ((key === "videoCodec" || key === "audioCodec" || key === "container") && value === "Any")
            return qsTr("Auto");
        if (key === "path")
            return folderName(value);
        return root.app.optionLabel(value);
    }
    function chosen(key, value) {
        if (key === "subtitles" || key === "audioTracks") {
            var values = root.app.settings[key] || [];
            return value === "-" ? values.length === 0 : values.indexOf(value) >= 0;
        }
        return String(root.rawValue(key, root.app.settings[key])) === String(value);
    }
    function select(key, value) {
        if (key === "subtitles" || key === "audioTracks") {
            var values = (root.app.settings[key] || []).slice();
            if (value === "-")
                values = [];
            else if (values.indexOf(value) >= 0)
                values.splice(values.indexOf(value), 1);
            else
                values.push(value);
            root.app.setting(key, values);
        } else {
            root.app.setting(key, value);
        }
    }

    component SettingItem: MenuItem {
        id: entry
        property bool keepOpen: false
        signal toggledInPlace()
        implicitHeight: 36
        leftPadding: 8
        rightPadding: 28
        font.pixelSize: 14
        implicitWidth: Math.max(200, Math.ceil(itemText.width) + leftPadding + rightPadding)
        TextMetrics { id: itemText; text: entry.text; font: entry.font }
        contentItem: RowLayout {
            Label {
                text: entry.text
                color: root.app.theme.text
                font: entry.font
                verticalAlignment: Text.AlignVCenter
                elide: Text.ElideRight
                Layout.fillWidth: true
            }
            Label {
                text: entry.subMenu ? entry.subMenu.selectedValue || "" : ""
                visible: text.length > 0
                color: root.app.theme.secondary
                font.pixelSize: 12
                padding: 4
                background: Rectangle { color: root.app.theme.hover; radius: 2 }
            }
        }
        indicator: Label {
            x: entry.width - width - 8
            y: (entry.height - height) / 2
            text: entry.checked ? "✓" : ""
            color: root.app.theme.text
        }
        background: Rectangle {
            color: entry.highlighted ? root.app.theme.hover : root.app.theme.popup
        }
        MouseArea {
            anchors.fill: parent
            enabled: entry.keepOpen
            acceptedButtons: Qt.LeftButton
            onClicked: entry.toggledInPlace()
        }
        Keys.onSpacePressed: function (event) {
            if (keepOpen)
                toggledInPlace();
            event.accepted = keepOpen;
        }
    }
    component SettingMenu: Menu {
        property string selectedValue: ""
        implicitWidth: {
            var widest = 200;
            for (var index = 0; index < count; index++) {
                var item = itemAt(index);
                if (item)
                    widest = Math.max(widest, item.implicitWidth);
            }
            return Math.ceil(Math.min(400, widest));
        }
        padding: 0
        margins: 4
        overlap: -4
        height: Math.min(implicitHeight, Math.max(100, root.app.height - 40))
        delegate: SettingItem {}
        background: Rectangle {
            radius: 2
            color: root.app.theme.popup
            border.color: root.app.theme.border
            border.width: 1
            layer.enabled: true
            layer.effect: DropShadow {
                color: Qt.rgba(0, 0, 0, 0.1)
                horizontalOffset: 0
                verticalOffset: 5
                radius: 8
            }
        }
    }
    Component {
        id: languageDivider
        MenuSeparator {}
    }
    component Choice: ComboBox {
        id: choice
        required property string caption
        required property string settingKey
        property var sections: []
        property bool folderChoice: false
        property bool containerChoice: false
        Layout.alignment: Qt.AlignVCenter
        Layout.preferredWidth: implicitWidth
        Layout.minimumWidth: Math.ceil(captionLabel.implicitWidth) + 24
        Layout.maximumWidth: implicitWidth
        implicitHeight: 20
        implicitWidth: Math.ceil(captionLabel.implicitWidth) + Math.ceil(valueLabel.implicitWidth) + 30
        padding: 0
        spacing: 0
        currentIndex: model.indexOf(root.rawValue(settingKey, root.app.settings[settingKey]))
        displayText: folderChoice ? root.folderName(root.app.settings.path) : containerChoice && root.app.settings.containerOSAlias ? root.app.settings.containerOSAlias : root.label(settingKey, currentIndex < 0 ? root.rawValue(settingKey, root.app.settings[settingKey]) : currentText)
        onActivated: root.select(settingKey, currentText)
        Accessible.name: caption + ": " + displayText
        Keys.onPressed: function (event) {
            if (event.key === Qt.Key_Down || event.key === Qt.Key_Space) {
                popup.open();
                event.accepted = true;
            }
        }
        ToolTip.visible: hovered && folderChoice
        ToolTip.text: root.app.settings.path || ""
        contentItem: RowLayout {
            spacing: 0
            Label {
                id: captionLabel
                text: choice.caption
                color: root.app.theme.secondary
                font.pixelSize: 14
            }
            Label {
                id: valueLabel
                text: choice.displayText
                color: root.app.theme.text
                font.pixelSize: 14
                elide: choice.folderChoice ? Text.ElideLeft : Text.ElideRight
                Layout.fillWidth: true
                Layout.leftMargin: 6
            }
            ColorImage {
                source: root.app.asset(choice.down ? "arrow-up.png" : "arrow-down.png", choice.down ? "icons/arrow-up.svg" : "icons/arrow-down.svg")
                color: root.app.theme.secondary
                defaultColor: "#000000"
                Layout.leftMargin: 4
                Layout.preferredWidth: 20
                Layout.preferredHeight: 20
            }
        }
        indicator: Item {}
        background: Rectangle {
            x: -6
            y: -6
            width: parent.width + 12
            height: parent.height + 12
            radius: 2
            color: choice.hovered || choice.visualFocus ? root.app.theme.hover : "transparent"
        }
        popup: SettingMenu {
            id: menu
            objectName: choice.objectName + "Menu"
            y: choice.height + 8
            x: -6
            Instantiator {
                model: choice.model
                delegate: SettingItem {
                    required property int index
                    required property string modelData
                    text: root.label(choice.settingKey, modelData)
                    checkable: true
                    checked: root.chosen(choice.settingKey, modelData)
                    onTriggered: {
                        choice.currentIndex = index;
                        choice.activated(index);
                    }
                }
                onObjectAdded: (index, item) => menu.insertItem(index, item)
                onObjectRemoved: (index, item) => menu.removeItem(item)
            }
            MenuSeparator { visible: choice.sections.length > 0 || choice.folderChoice || choice.containerChoice }
            Instantiator {
                model: choice.sections
                delegate: SettingMenu {
                    id: section
                    required property var modelData
                    property bool dividersAdded: false
                    title: modelData.title
                    selectedValue: ["subtitles", "audioTracks"].indexOf(modelData.key) < 0 ? root.label(modelData.key, root.rawValue(modelData.key, root.app.settings[modelData.key])) : ""
                    onAboutToShow: {
                        if (dividersAdded)
                            return;
                        dividersAdded = true;
                        if (modelData.key === "subtitles") {
                            insertItem(2, languageDivider.createObject(section.contentItem));
                            insertItem(8, languageDivider.createObject(section.contentItem));
                        } else if (modelData.key === "audioTracks") {
                            insertItem(3, languageDivider.createObject(section.contentItem));
                        }
                    }
                    Instantiator {
                        model: section.modelData.values
                        delegate: SettingItem {
                            required property string modelData
                            keepOpen: ["subtitles", "audioTracks"].indexOf(section.modelData.key) >= 0
                            text: root.label(section.modelData.key, modelData)
                            checkable: true
                            checked: root.chosen(section.modelData.key, modelData)
                            onTriggered: root.select(section.modelData.key, modelData)
                            onToggledInPlace: root.select(section.modelData.key, modelData)
                        }
                        onObjectAdded: (index, item) => section.insertItem(index, item)
                        onObjectRemoved: (index, item) => section.removeItem(item)
                    }
                }
                onObjectAdded: (index, item) => menu.addMenu(item)
                onObjectRemoved: (index, item) => menu.removeMenu(item)
            }
            Instantiator {
                model: choice.containerChoice ? ["Windows", "Mac OS", "Linux", "iOS", "Android"] : []
                delegate: SettingItem {
                    required property string modelData
                    text: modelData
                    checkable: true
                    checked: root.app.settings.containerOSAlias === modelData
                    onTriggered: root.app.setting("containerOSAlias", modelData)
                }
                onObjectAdded: (index, item) => menu.addItem(item)
                onObjectRemoved: (index, item) => menu.removeItem(item)
            }
            Instantiator {
                model: choice.folderChoice ? 1 : 0
                delegate: SettingItem {
                    text: qsTr("Browse") + "…"
                    onTriggered: root.browseRequested()
                }
                onObjectAdded: (index, item) => menu.addItem(item)
                onObjectRemoved: (index, item) => menu.removeItem(item)
            }
        }
    }

    Choice {
        objectName: "smartMediaType"
        caption: qsTr("Download")
        settingKey: "mediaType"
        model: ["video", "audio"]
        sections: root.audio ? [] : [
            {title: qsTr("Subtitles"), key: "subtitles", values: Languages.subtitles.map(function (item) { return item.language; })},
            {title: qsTr("Audio tracks"), key: "audioTracks", values: Languages.audioTracks.map(function (item) { return item.language; })}
        ]
    }
    Choice {
        objectName: "smartQuality"
        caption: qsTr("Quality")
        settingKey: root.audio ? "audioBitrate" : "quality"
        model: root.audio ? ["Original", "320kbps", "256kbps", "128kbps", "64kbps"] : ["Best", "8K", "4K", "1440p", "1080p", "720p", "480p", "360p", "240p", "144p"]
        sections: root.audio ? [{title: qsTr("Codec"), key: "audioCodec", values: root.app.settings.container === "mp3" ? ["Any", "MP3"] : root.app.settings.container === "ogg" ? ["Any", "Vorbis", "Opus"] : ["Any", "AAC"]}] : [
            {title: qsTr("Frame Rate"), key: "frameRate", values: ["Maximum", "240", "120", "60", "50", "30", "25"]},
            {title: qsTr("Codec"), key: "videoCodec", values: root.app.settings.container === "mp4" ? ["Any", "H264", "H265", "AV1"] : root.app.settings.container === "mkv" ? ["Any", "H264", "H265", "VP8", "VP9"] : ["Any"]},
            {title: qsTr("VR"), key: "videoVR", values: ["Any", "2D/360°", "3D/180°", "3D/360°", "None"]}
        ]
    }
    Choice {
        objectName: "smartContainer"
        caption: root.app.settings.containerOSAlias ? qsTr("For") : qsTr("Format")
        settingKey: "container"
        containerChoice: true
        model: root.audio ? ["M4A", "MP3", "OGG"] : ["MKV", "MP4"]
    }
    Choice {
        objectName: "smartFolder"
        caption: qsTr("Save To")
        settingKey: "path"
        folderChoice: true
        model: [root.app.settings.path || ""].concat(root.app.settings.paths || []).filter(function (path, index, values) { return values.indexOf(path) === index; })
        Layout.fillWidth: true
    }
    Item { Layout.fillWidth: true }
}
