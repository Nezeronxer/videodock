pragma ComponentBehavior: Bound
import Qt.labs.platform as Platform
import QtQuick
import QtQuick.Controls
import QtQuick.Controls.impl
import QtQuick.Dialogs
import QtQuick.Layouts
import VideoDock
import Qt5Compat.GraphicalEffects

ApplicationWindow {
    id: window

    component AppMenu: Menu {
        implicitWidth: {
            var widest = 200;
            for (var i = 0; i < count; ++i) {
                var entry = itemAt(i);
                if (entry && entry.contentItem)
                    widest = Math.max(widest, entry.contentItem.implicitWidth + (entry.leftPadding || 0) + (entry.rightPadding || 0) + 24);
            }
            return Math.ceil(widest + leftPadding + rightPadding);
        }
    }

    readonly property alias theme: colors
    readonly property color green: colors.accent
    property bool searchVisible: false

    UiTheme {
        id: colors
        mode: window.settings.theme || "dark"
    }
    property var state: ({
            "settings": {},
            "jobs": [],
            "subscriptions": [],
            "account": {}
        })
    property var settings: state.settings || ({})
    property string activeTab: "All"
    property string search: ""
    property string sortKey: "date"
    property bool descending: true
    property bool browserVisible: false
    readonly property bool hasMedia: (state.jobs || []).length + (state.subscriptions || []).length > 0
    property string localNotice: ""
    property string lastNotification: ""
    property bool quitConfirmed: false
    property bool explicitQuit: false
    property bool introConsidered: false
    property bool geometryRestored: false
    property var selectedIds: []
    property var focusedJob: null
    property bool manualDismissed: false

    function tabTitle(key) {
        return ({
                All: qsTr("All"),
                Video: qsTr("Video"),
                Audio: qsTr("Audio"),
                Playlists: qsTr("Playlists"),
                Channels: qsTr("Channels"),
                Subscriptions: qsTr("Subscriptions")
            })[key] || key;
    }
    function statusTitle(key) {
        return ({
                queued: qsTr("Queued"),
                parsing: qsTr("Retrieving information"),
                downloading: qsTr("Downloading"),
                processing: qsTr("Processing"),
                paused: qsTr("Paused"),
                complete: qsTr("Complete"),
                error: qsTr("Error")
            })[key] || key;
    }
    function optionText(key, value) {
        if (key === "quality" && typeof value === "number")
            return value === 0 ? "Best" : value === 4320 ? "8K" : value === 2160 ? "4K" : value + "p";
        if (key === "container")
            return String(value || "").toUpperCase();
        if (key === "videoCodec" || key === "audioCodec")
            return String(value || "").toLowerCase() === "any" ? "Any" : String(value || "").toUpperCase();
        if (key === "frameRate" && value === 0)
            return "Any";
        if (key === "videoVR" && value === false)
            return "2D";
        if (key === "speedLimit" && typeof value === "number") {
            if (value === 0)
                return "Unlimited";
            if (value % 125000 === 0)
                return (value / 125000) + " Mbps";
            if (value % 125 === 0)
                return (value / 125) + " kbps";
            return value + " B/s";
        }
        if (key === "subscriptionUpdatePeriod" && typeof value === "number")
            return ({
                    300: "5 min",
                    900: "15 min",
                    1800: "30 min",
                    3600: "1 hour",
                    10800: "3 hours",
                    21600: "6 hours",
                    43200: "12 hours",
                    86400: "1 day"
                })[value] || String(value);
        return String(value === undefined ? "" : value);
    }
    function platformIcon(url) {
        var match = /^https?:\/\/(?:[^/@\s]+@)?([^/:?#\s]+)/i.exec(String(url || ""));
        if (!match)
            return "";
        var host = match[1].toLowerCase();
        var services = {
            "youtube.com": "youtube", "youtu.be": "youtube", "youtube-nocookie.com": "youtube",
            "tiktok.com": "tiktok", "vimeo.com": "vimeo", "soundcloud.com": "soundcloud",
            "facebook.com": "facebook", "fb.watch": "facebook", "twitch.tv": "twitch",
            "dailymotion.com": "dailymotion", "dai.ly": "dailymotion",
            "bilibili.com": "bilibili", "b23.tv": "bilibili", "rumble.com": "rumble",
            "tv.naver.com": "naver", "tv.naver.tv": "naver", "tumblr.com": "tumblr",
            "nicovideo.jp": "nicovideo", "nico.ms": "nicovideo",
            "likee.video": "likee", "likee.com": "likee", "likeevideo.ru": "likee",
            "flickr.com": "flickr", "flic.kr": "flickr", "bitchute.com": "bitchute"
        };
        for (var domain in services) {
            if (host === domain || host.endsWith("." + domain)) {
                var service = services[domain];
                return asset((service === "tiktok" ? "tik-tok" : service) + "-icon.svg", "icons/service-" + service + ".svg");
            }
        }
        return "icons/browser.svg";
    }

    function asset(name, fallback) {
        var source = typeof backend.referenceAsset === "function" ? backend.referenceAsset(name) : "";
        return source || fallback || "";
    }
    function optionLabel(value) {
        return ({
            video: qsTr("Video"), audio: qsTr("Audio"), playlist: qsTr("Playlist"), channel: qsTr("Channel"), Best: qsTr("Best"),
            Any: qsTr("Any"), Unlimited: qsTr("Unlimited"), en: "English", ru: "Русский",
            Original: qsTr("Original"), unknown: qsTr("Unknown"), Default: qsTr("Default"),
            Maximum: qsTr("Highest"),
            "5 min": qsTr("Every 5 minutes"), "15 min": qsTr("Every 15 minutes"),
            "30 min": qsTr("Every 30 minutes"), "1 hour": qsTr("Every hour"),
            "3 hours": qsTr("Every 3 hours"), "6 hours": qsTr("Every 6 hours"),
            "12 hours": qsTr("Every 12 hours"), "1 day": qsTr("Every day")
        })[value] || value;
    }
    function send(action) {
        var response;
        try {
            response = JSON.parse(backend.dispatch(JSON.stringify(action)));
        } catch (error) {
            localNotice = String(error);
            return ({
                    "error": localNotice
                });
        }
        if (!response || typeof response !== "object")
            response = ({
                    result: response
                });
        if (response && response.error)
            localNotice = response.error;
        else
            localNotice = "";
        if (response && response.url) {
            browserVisible = true;
            browser.go(response.url);
        }
        if (response && response.clipboard) {
            clipboardField.text = response.clipboard;
            clipboardField.selectAll();
            clipboardField.copy();
            clipboardField.clear();
        }
        return response;
    }

    function setting(key, value) {
        var values = {};
        values[key] = value;
        send({
            "action": "settings",
            "values": values
        });
    }

    function jobs(command, ids, deleteFiles) {
        return send({
            "action": "jobs",
            "command": command,
            "ids": ids || selectedIds,
            "delete_files": !!deleteFiles
        });
    }

    function pasteLink() {
        clipboardField.clear();
        clipboardField.paste();
        var value = clipboardField.text.trim();
        clipboardField.clear();
        if (/^https?:\/\//i.test(value))
            parse(value);
        else {
            urlInput.text = "";
            urlDialog.open();
        }
    }

    function parse(url) {
        browserVisible = false;
        manualDismissed = false;
        activeTab = "All";
        search = "";
        sortKey = "date";
        descending = true;
        send({
            "action": "parse",
            "url": url
        });
    }

    function selectedRows() {
        return (state.jobs || []).filter(function (j) {
            return selectedIds.indexOf(j.id) >= 0;
        });
    }

    function chooseJob(job, multiple) {
        focusedJob = job;
        if (multiple) {
            let copy = selectedIds.slice();
            let i = copy.indexOf(job.id);
            if (i < 0)
                copy.push(job.id);
            else
                copy.splice(i, 1);
            selectedIds = copy;
        } else {
            selectedIds = [job.id];
        }
    }

    function viewDetails(job) {
        focusedJob = job;
        detailsDialog.open();
    }

    function nativePath(url) {
        var path = decodeURIComponent(String(url).replace(/^file:\/\//, ""));
        return Qt.platform.os === "windows" ? path.replace(/^\/([A-Za-z]:)/, "$1") : path;
    }
    function requestQuit() {
        explicitQuit = true;
        close();
    }

    function confirmRemove() {
        if (selectedIds.length)
            removeDialog.open();
    }

    function listRows() {
        var rows = activeTab === "Subscriptions" ? (state.subscriptions || []) : (state.jobs || []);
        return rows.filter(function (j) {
            var matches = activeTab === "All" || activeTab === "Subscriptions" || (activeTab === "Video" && j.kind === "video") || (activeTab === "Audio" && j.kind === "audio") || (activeTab === "Playlists" && j.kind === "playlist") || (activeTab === "Channels" && j.kind === "channel");
            return matches && (!search || ((j.title || "") + " " + (j.url || "")).toLowerCase().indexOf(search.toLowerCase()) >= 0);
        }).map(function (job, order) {
            return {job: job, order: order};
        }).sort(function (a, b) {
            var orderA = Number(a.job.added_order === undefined ? a.job.created_at : a.job.added_order) || 0;
            var orderB = Number(b.job.added_order === undefined ? b.job.created_at : b.job.added_order) || 0;
            var result = sortKey === "name" ? String(a.job.title || "").localeCompare(String(b.job.title || "")) : orderA - orderB;
            if (!result)
                result = a.order - b.order;
            return descending ? -result : result;
        }).map(function (entry) {
            return entry.job;
        });
    }

    objectName: "mainWindow"
    visible: true
    width: 1150
    height: 618
    minimumWidth: 950
    minimumHeight: 568
    title: "VideoDock"
    color: theme.canvas
    font.pixelSize: 14
    font.family: "Sans Serif"
    palette.window: theme.window
    palette.base: theme.input
    palette.alternateBase: theme.hover
    palette.button: theme.button
    palette.buttonText: theme.text
    palette.highlight: theme.accent
    palette.highlightedText: theme.text
    palette.text: theme.text
    palette.windowText: theme.text
    palette.placeholderText: theme.secondary
    palette.mid: theme.border
    palette.dark: theme.border
    palette.brightText: theme.accentText
    palette.light: theme.hover
    palette.midlight: theme.button
    palette.shadow: theme.list
    palette.toolTipBase: theme.popup
    palette.toolTipText: theme.text
    palette.link: theme.accent
    palette.disabled.text: theme.disabled
    palette.disabled.windowText: theme.disabled
    palette.disabled.buttonText: theme.disabled
    palette.disabled.highlight: theme.border
    palette.disabled.dark: theme.button
    palette.disabled.brightText: theme.disabled
    onClosing: function (close) {
        if (settings.systemTrayIconVisible && tray.available && !quitConfirmed && !explicitQuit) {
            close.accepted = false;
            window.hide();
            return;
        }
        if (!quitConfirmed && settings.askQuit !== false && (state.jobs || []).some(function (j) {
            return ["queued", "downloading", "parsing"].indexOf(j.state) >= 0;
        })) {
            close.accepted = false;
            quitDialog.open();
        }
        if (close.accepted && geometryRestored)
            window.send({action: "settings", values: {windowWidth: Math.round(window.width), windowHeight: Math.round(window.height)}});
    }

    Backend {
        id: backend

        onSnapshotChanged: {
            try {
                window.state = JSON.parse(snapshot);
                if (!window.geometryRestored && window.state.settings) {
                    window.geometryRestored = true;
                    window.width = window.settings.windowWidth || 1150;
                    window.height = window.settings.windowHeight || 618;
                }
            } catch (error) {
                window.localNotice = String(error);
            }
            if (window.state.manual && !window.manualDismissed && !manualDialog.visible)
                manualDialog.open();

            if (!window.state.manual && manualDialog.visible)
                manualDialog.close();

            var notification = window.state.notification;
            if (!window.introConsidered && window.state.onboarding_completed === false) {
                window.introConsidered = true;
                Qt.callLater(function () {
                    onboarding.open();
                });
            }
            if (notification && notification.id !== window.lastNotification) {
                window.lastNotification = notification.id;
                if (window.settings.systemNotificationsEnabled !== false && tray.available)
                    tray.showMessage(notification.title || "VideoDock", notification.body || "");
            }
        }
        Component.onCompleted: {
            initializeProfile();
            refresh();
        }
    }

    Timer {
        interval: 250
        running: true
        repeat: true
        onTriggered: backend.refresh()
    }

    TextEdit {
        id: clipboardField
        objectName: "clipboardBuffer"
        Accessible.ignored: true
        visible: false
    }

    Shortcut {
        sequences: [StandardKey.Paste]
        onActivated: window.pasteLink()
    }

    Shortcut {
        sequence: "Ctrl+F"
        onActivated: {
            window.searchVisible = true;
            searchField.forceActiveFocus();
        }
    }

    Shortcut {
        sequence: "Delete"
        onActivated: window.confirmRemove()
    }

    Shortcut {
        sequence: "Ctrl+A"
        onActivated: window.selectedIds = window.listRows().map(function (j) {
            return j.id;
        })
    }

    Shortcut {
        sequence: "F1"
        onActivated: onboarding.open()
    }

    ColumnLayout {
        anchors.fill: parent
        spacing: 0

        Rectangle {
            objectName: "mainToolbar"
            Layout.fillWidth: true
            implicitHeight: 64
            color: window.hasMedia ? window.theme.toolbar : window.theme.canvas

            RowLayout {
                anchors.fill: parent
                anchors.leftMargin: 16
                anchors.rightMargin: 24
                spacing: 16

                Button {

                    palette.dark: window.theme.accent
                    objectName: "pasteButton"
                    text: qsTr("Paste Link")
                    icon.source: "icons/connection.svg"
                    icon.color: window.theme.accentText
                    icon.width: 20
                    icon.height: 20
                    font.pixelSize: 14
                    highlighted: true
                    Layout.preferredWidth: window.width < 800 ? 40 : 180
                    display: window.width < 800 ? AbstractButton.IconOnly : AbstractButton.TextBesideIcon
                    Layout.preferredHeight: 40
                    onClicked: window.pasteLink()
                }

                Switch {
                    id: smartSwitch

                    palette.dark: window.theme.accent
                    objectName: "smartModeSwitch"
                    padding: 0
                    indicator: Rectangle {
                        implicitWidth: 40
                        implicitHeight: 24
                        radius: 12
                        color: smartSwitch.checked ? window.green : window.theme.button
                        border.width: smartSwitch.visualFocus ? 2 : 0
                        border.color: window.theme.text
                        Rectangle {
                            x: smartSwitch.checked ? 18 : 2
                            y: 2
                            width: 20
                            height: 20
                            radius: 10
                            color: window.theme.accentText
                            Behavior on x { NumberAnimation { duration: 150 } }
                            ColorImage {
                                anchors.centerIn: parent
                                width: 16
                                height: 16
                                source: window.asset("smart-mode.png", "icons/power.svg")
                                color: smartSwitch.checked ? window.green : window.theme.secondary
                                defaultColor: "#000000"
                            }
                        }
                    }
                    Layout.preferredWidth: 40
                    Layout.preferredHeight: 24
                    checked: window.settings.smartMode === true
                    onToggled: window.setting("smartMode", checked)
                    Accessible.name: qsTr("Smart Mode")
                    ToolTip.text: qsTr("Smart Mode")
                    ToolTip.visible: hovered
                }

                SmartModeBar {
                    objectName: "smartModeBar"
                    visible: window.settings.smartMode === true
                    Layout.fillWidth: true
                    Layout.alignment: Qt.AlignVCenter
                    Layout.preferredHeight: 20
                    Layout.maximumHeight: 20
                    app: window
                    onBrowseRequested: folderDialog.open()
                }

                Label {
                    visible: window.settings.smartMode !== true
                    text: qsTr("Enable Smart Mode to download links in one click")
                    color: window.theme.secondary
                    Layout.fillWidth: true
                    wrapMode: Text.Wrap
                }

                ToolButton {
                    objectName: "preferencesButton"
                    Layout.preferredWidth: 28
                    Layout.preferredHeight: 28
                    text: qsTr("Preferences")
                    display: AbstractButton.IconOnly
                    palette.button: "transparent"
                    icon.source: window.asset("settings.png", "icons/settings.svg")
                    Accessible.name: text
                    ToolTip.text: text
                    ToolTip.visible: hovered
                    onClicked: settingsDialog.open()
                }
                ToolButton {
                    objectName: "controlPanelOpenerButton"
                    Layout.preferredWidth: 28
                    Layout.preferredHeight: 28
                    Layout.leftMargin: -8
                    text: qsTr("Control Panel")
                    display: AbstractButton.IconOnly
                    palette.button: "transparent"
                    icon.source: window.asset("control-panel.png", "icons/panel.svg")
                    Accessible.name: text
                    ToolTip.text: text
                    ToolTip.visible: hovered
                    onClicked: controlPanel.open()
                }
            }
        }

        Item {
            Layout.fillWidth: true
            Layout.fillHeight: true

            ColumnLayout {
                anchors.fill: parent
                spacing: 0
                Rectangle {
                    objectName: "mediaListHeader"
                    visible: window.hasMedia
                    Layout.fillWidth: true
                    implicitHeight: 64
                    color: window.theme.window

                    RowLayout {
                        anchors.fill: parent
                        anchors.leftMargin: 16
                        anchors.rightMargin: 16
                        anchors.topMargin: 12
                        anchors.bottomMargin: 12
                        spacing: 0

                        Repeater {
                            model: ["All", "Video", "Audio", "Playlists", "Channels", "Subscriptions"]
                            Button {
                                id: filterTab
                                palette.dark: window.theme.accent
                                required property string modelData
                                text: window.tabTitle(modelData)
                                visible: !window.searchVisible || window.width >= 1100 || checked
                                flat: true
                                checked: window.activeTab === modelData
                                checkable: true
                                leftPadding: 8
                                rightPadding: leftPadding
                                implicitWidth: Math.ceil(tabWidth.width) + 20
                                font.weight: checked ? Font.DemiBold : Font.Normal
                                TextMetrics {
                                    id: tabWidth
                                    text: filterTab.text
                                    font.pixelSize: 14
                                    font.weight: Font.DemiBold
                                }
                                onClicked: {
                                    window.activeTab = modelData;
                                    window.selectedIds = [];
                                }
                                contentItem: Label {
                                    textFormat: Text.PlainText
                                    text: filterTab.text
                                    color: filterTab.checked ? window.theme.text : window.theme.secondary
                                    horizontalAlignment: Text.AlignHCenter
                                    verticalAlignment: Text.AlignVCenter
                                    font: filterTab.font
                                }
                                background: Rectangle {
                                    color: filterTab.hovered ? window.theme.hover : "transparent"
                                    Rectangle {
                                        anchors.bottom: parent.bottom
                                        width: parent.width
                                        height: 2
                                        color: window.green
                                        visible: filterTab.checked
                                    }
                                }
                            }
                        }

                        Item {
                            Layout.fillWidth: true
                        }
                        Label {
                            readonly property int count: window.listRows().length
                            text: count === 1 ? qsTr("1 item") : qsTr("%n items", "", count)
                            color: window.theme.secondary
                            visible: !window.searchVisible
                            font.pixelSize: 12
                        }
                        TextField {
                            id: searchField
                            objectName: "searchField"
                            visible: window.searchVisible
                            Layout.preferredWidth: Math.min(200, window.width * 0.18)
                            placeholderText: qsTr("Search downloads")
                            onTextChanged: window.search = text
                            selectByMouse: true
                            Keys.onEscapePressed: {
                                clear();
                                window.searchVisible = false;
                            }
                        }
                        ToolButton {
                            text: window.searchVisible ? qsTr("Close search") : qsTr("Search downloads")
                            display: AbstractButton.IconOnly
                            palette.button: "transparent"
                            icon.source: window.searchVisible ? "icons/close.svg" : "icons/search.svg"
                            Accessible.name: text
                            onClicked: {
                                window.searchVisible = !window.searchVisible;
                                if (window.searchVisible)
                                    searchField.forceActiveFocus();
                                else
                                    searchField.clear();
                            }
                        }
                        ToolButton {
                            objectName: "sortButton"
                            text: qsTr("Sort")
                            display: AbstractButton.IconOnly
                            palette.button: "transparent"
                            icon.source: "icons/sort.svg"
                            Accessible.name: text
                            onClicked: sortMenu.popup()
                            AppMenu {
                                id: sortMenu
                                palette.window: window.theme.popup
                                objectName: "sortMenu"
                                MenuItem {
                                    text: qsTr("Date Added")
                                    checkable: true
                                    checked: window.sortKey === "date"
                                    onTriggered: window.sortKey = "date"
                                }
                                MenuItem {
                                    text: qsTr("Name")
                                    checkable: true
                                    checked: window.sortKey === "name"
                                    onTriggered: window.sortKey = "name"
                                }
                                MenuSeparator {}
                                MenuItem {
                                    text: qsTr("Ascending")
                                    checkable: true
                                    checked: !window.descending
                                    onTriggered: window.descending = false
                                }
                                MenuItem {
                                    text: qsTr("Descending")
                                    checkable: true
                                    checked: window.descending
                                    onTriggered: window.descending = true
                                }
                            }
                        }
                    }
                }

                Rectangle {
                    Layout.fillWidth: true
                    Layout.fillHeight: true
                    visible: window.hasMedia
                    color: window.theme.list
                    JobList {
                        app: window
                        rows: window.listRows()
                        anchors.fill: parent
                    }
                }

                Item {
                    Layout.fillWidth: true
                    Layout.preferredHeight: 88
                    visible: window.hasMedia
                }
            }

            ColumnLayout {
                visible: !window.hasMedia && !window.browserVisible
                objectName: "browserHomeTitle"
                anchors.horizontalCenter: parent.horizontalCenter
                y: Math.max(20, (parent.height - height - browser.height) / 2)
                width: Math.min(594, parent.width - 40)
                spacing: 0
                Label {
                    text: qsTr("Find videos to download right in the app")
                    color: window.theme.secondary
                    font.pixelSize: 14
                    Layout.fillWidth: true
                    horizontalAlignment: Text.AlignHCenter
                }
                Label {
                    text: qsTr("Click the service icon below and start searching")
                    font.pixelSize: 28
                    Layout.fillWidth: true
                    wrapMode: Text.Wrap
                    horizontalAlignment: Text.AlignHCenter
                }
            }

            BrowserPane {
                id: browser
                objectName: "browserPane"
                app: window
                anchors.horizontalCenter: parent.horizontalCenter
                anchors.bottom: parent.bottom
                anchors.bottomMargin: 16
                width: window.browserVisible ? parent.width - 32 : Math.min(parent.width - 34, 1080)
                height: window.browserVisible ? parent.height - 20 : window.hasMedia ? 56 : 330
                z: 3
            }
        }

        Rectangle {
            Layout.fillWidth: true
            implicitHeight: noticeLabel.text ? Math.min(90, noticeLabel.implicitHeight + 22) : 0
            color: window.theme.notice
            visible: implicitHeight > 0

            RowLayout {
                anchors.fill: parent
                anchors.margins: 10

                Label {
                    id: noticeLabel
                    textFormat: Text.PlainText

                    text: window.localNotice || window.state.notice || ""
                    Layout.fillWidth: true
                    wrapMode: Text.Wrap
                    color: window.theme.noticeText
                }

                ToolButton {
                    text: qsTr("Close")
                    display: AbstractButton.IconOnly
                    palette.button: "transparent"
                    icon.source: "icons/close.svg"
                    onClicked: {
                        window.localNotice = "";
                        window.send({
                            "action": "app",
                            "command": "clear_notice"
                        });
                    }
                }
            }
        }
    }

    FolderDialog {
        id: folderDialog

        title: qsTr("Save To")
        onAccepted: window.setting("path", window.nativePath(selectedFolder))
    }

    SettingsDialog {
        id: settingsDialog

        app: window
    }

    ManualDialog {
        id: manualDialog

        app: window
    }

    TransferDialog {
        id: transfer

        app: window
    }

    DetailsDialog {
        id: detailsDialog

        app: window
    }

    OnboardingDialog {
        id: onboarding

        app: window
    }

    AppDialog {
        id: urlDialog
        app: window
        objectName: "urlDialog"

        title: qsTr("Paste Link")
        modal: true
        anchors.centerIn: parent
        width: 620
        standardButtons: Dialog.Ok | Dialog.Cancel
        onAccepted: window.parse(urlInput.text)

        TextField {
            id: urlInput

            anchors.fill: parent
            placeholderText: "https://…"
            selectByMouse: true
            onAccepted: {
                window.parse(text);
                urlDialog.close();
            }
        }
    }

    AppDialog {
        id: removeDialog
        app: window
        objectName: "removeDialog"
        property var pendingIds: []

        title: qsTr("Delete Selected Items")
        modal: true
        anchors.centerIn: parent
        width: 440
        standardButtons: Dialog.Ok | Dialog.Cancel
        onOpened: {
            pendingIds = window.selectedIds.slice();
            deleteFiles.checked = false;
        }
        onAccepted: {
            var ids = pendingIds;
            pendingIds = [];
            if (ids.length)
                window.jobs("remove", ids, deleteFiles.checked);
            window.selectedIds = [];
        }

        ColumnLayout {
            width: parent.width

            Label {
                textFormat: Text.PlainText
                text: qsTr("Remove %1 item(s) from the list?").arg(removeDialog.pendingIds.length)
                wrapMode: Text.Wrap
                Layout.fillWidth: true
            }

            CheckBox {
                id: deleteFiles
                objectName: "deleteFilesCheckbox"

                text: qsTr("Also delete files from device")
                checked: false
            }
        }
    }

    ControlPanel {
        id: controlPanel
        app: window
        onPreferencesRequested: settingsDialog.open()
    }

    AppDialog {
        id: about
        app: window
        objectName: "about"

        title: qsTr("About VideoDock")
        anchors.centerIn: parent
        modal: true
        width: 440
        standardButtons: Dialog.Close

        Label {
            textFormat: Text.PlainText
            text: qsTr("VideoDock\nRust download engine · Qt interface\nIndependent implementation.")
            padding: 18
            wrapMode: Text.Wrap
        }
    }

    AppDialog {
        id: quitDialog
        app: window
        objectName: "quitDialog"

        title: qsTr("Downloads are still running")
        anchors.centerIn: parent
        modal: true
        width: 440
        standardButtons: Dialog.Yes | Dialog.No
        onAccepted: {
            window.quitConfirmed = true;
            window.close();
        }

        Label {
            textFormat: Text.PlainText
            text: qsTr("Quit with incomplete downloads?")
            padding: 16
        }
    }

    Platform.SystemTrayIcon {
        id: tray

        visible: available && window.settings.systemTrayIconVisible === true
        icon.name: "folder-download"
        tooltip: "VideoDock"
        onActivated: {
            window.show();
            window.raise();
            window.requestActivate();
        }

        menu: Platform.Menu {
            Platform.MenuItem {
                text: qsTr("Show VideoDock")
                onTriggered: window.show()
            }

            Platform.MenuItem {
                text: qsTr("Pause All")
                onTriggered: window.jobs("pause", [])
            }

            Platform.MenuItem {
                text: qsTr("Resume All")
                onTriggered: window.jobs("resume", [])
            }

            Platform.MenuItem {
                text: qsTr("Quit")
                onTriggered: {
                    window.requestQuit();
                }
            }
        }
    }

    menuBar: MenuBar {
        background: Rectangle { color: window.theme.canvas }
        implicitHeight: 30
        height: 30
        delegate: MenuBarItem {
            implicitHeight: 30
            leftPadding: 8
            rightPadding: 8
            font.pixelSize: 14
        }
        AppMenu {
            palette.window: window.theme.popup
            title: qsTr("File")

            MenuItem {
                text: qsTr("Open Directory")
                onTriggered: window.send({
                    "action": "app",
                    "command": "open_directory"
                })
            }

            MenuSeparator {}

            MenuItem {
                text: qsTr("Import Downloads…")
                onTriggered: transfer.openMode("import")
            }

            MenuItem {
                text: qsTr("Export Downloads…")
                onTriggered: transfer.openMode("export")
            }

            MenuItem {
                text: qsTr("Export Subscriptions…")
                onTriggered: transfer.openMode("subscriptions")
            }

            MenuSeparator {}

            MenuItem {
                text: qsTr("Quit")
                onTriggered: window.requestQuit()
            }
        }

        AppMenu {

            palette.window: window.theme.popup
            title: qsTr("Edit")

            MenuItem {
                text: qsTr("Paste Link")
                onTriggered: window.pasteLink()
            }

            AppMenu {

                palette.window: window.theme.popup
                title: qsTr('Download "Liked"')

                MenuItem {
                    text: qsTr("Download Playlist")
                    onTriggered: window.send({
                        "action": "account",
                        "command": "liked"
                    })
                }

                MenuItem {
                    text: qsTr("Subscribe to Playlist")
                    onTriggered: window.send({
                        "action": "account",
                        "command": "liked",
                        "subscribe": true
                    })
                }
            }

            AppMenu {

                palette.window: window.theme.popup
                title: qsTr('Download "Watch Later"')

                MenuItem {
                    text: qsTr("Download Playlist")
                    onTriggered: window.send({
                        "action": "account",
                        "command": "watch_later"
                    })
                }

                MenuItem {
                    text: qsTr("Subscribe to Playlist")
                    onTriggered: window.send({
                        "action": "account",
                        "command": "watch_later",
                        "subscribe": true
                    })
                }
            }

            MenuSeparator {}

            MenuItem {
                text: qsTr("Pause All")
                onTriggered: window.jobs("pause", [])
            }

            MenuItem {
                text: qsTr("Resume All")
                onTriggered: window.jobs("resume", [])
            }

            MenuItem {
                text: qsTr("Remove All…")
                onTriggered: {
                    window.selectedIds = (window.state.jobs || []).map(function (j) {
                        return j.id;
                    });
                    window.confirmRemove();
                }
            }
        }

        AppMenu {

            palette.window: window.theme.popup
            title: qsTr("View")
            objectName: "viewMenu"

            Repeater {
                model: ["All", "Video", "Audio", "Playlists", "Channels", "Subscriptions"]

                MenuItem {
                    required property string modelData

                    text: window.tabTitle(modelData)
                    checkable: true
                    checked: window.activeTab === modelData
                    onTriggered: window.activeTab = modelData
                }
            }

            MenuSeparator {}

            AppMenu {

                palette.window: window.theme.popup
                title: qsTr("Sort")

                MenuItem {
                    text: qsTr("Date Added")
                    checkable: true
                    checked: window.sortKey === "date"
                    onTriggered: window.sortKey = "date"
                }

                MenuItem {
                    text: qsTr("Name")
                    checkable: true
                    checked: window.sortKey === "name"
                    onTriggered: window.sortKey = "name"
                }

                MenuSeparator {}

                MenuItem {
                    text: qsTr("Ascending")
                    onTriggered: window.descending = false
                }

                MenuItem {
                    text: qsTr("Descending")
                    onTriggered: window.descending = true
                }
            }

            MenuItem {
                text: qsTr("Show Browser")
                checkable: true
                checked: window.browserVisible
                onTriggered: window.browserVisible = !window.browserVisible
            }

            AppMenu {
                objectName: "appearanceMenu"
                title: qsTr("Appearance")
                icon.source: "icons/appearance.svg"
                palette.window: window.theme.popup

                MenuItem {
                    objectName: "theme-dark"
                    text: qsTr("Dark")
                    checkable: true
                    checked: window.settings.theme === "dark"
                    onTriggered: window.setting("theme", "dark")
                }
                MenuItem {
                    objectName: "theme-light"
                    text: qsTr("Light")
                    checkable: true
                    checked: window.settings.theme === "light"
                    onTriggered: window.setting("theme", "light")
                }
                MenuItem {
                    objectName: "theme-system"
                    text: qsTr("System")
                    checkable: true
                    checked: window.settings.theme === "system"
                    onTriggered: window.setting("theme", "system")
                }
            }
        }

        AppMenu {

            palette.window: window.theme.popup
            title: qsTr("Tools")

            MenuItem {
                text: qsTr("Control Panel…")
                onTriggered: controlPanel.open()
            }

            MenuItem {
                text: qsTr("Preferences…")
                onTriggered: settingsDialog.open()
            }

            MenuItem {
                text: qsTr("Check For Updates…")
                onTriggered: window.send({
                    "action": "app",
                    "command": "check_updates"
                })
            }
        }

        AppMenu {

            palette.window: window.theme.popup
            title: qsTr("Help")

            MenuItem {
                text: qsTr("Getting Started")
                onTriggered: onboarding.open()
            }

            MenuItem {
                text: qsTr("Open Logs Folder")
                onTriggered: window.send({
                    "action": "app",
                    "command": "open_logs"
                })
            }

            MenuItem {
                text: qsTr("About")
                onTriggered: about.open()
            }
        }
    }
}
