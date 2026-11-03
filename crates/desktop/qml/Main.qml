pragma ComponentBehavior: Bound
import Qt.labs.platform as Platform
import QtQuick
import QtQuick.Controls
import QtQuick.Dialogs
import QtQuick.Layouts
import VideoDock

ApplicationWindow {
    id: window

    property color green: "#5CB400"
    property var state: ({
            "settings": {},
            "jobs": [],
            "subscriptions": [],
            "processing": [],
            "account": {}
        })
    property var settings: state.settings || ({})
    property string activeTab: "All"
    property string search: ""
    property string sortKey: "date"
    property bool descending: true
    property bool browserVisible: false
    property string localNotice: ""
    property string lastNotification: ""
    property bool quitConfirmed: false
    property bool explicitQuit: false
    property bool introConsidered: false
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
                Subscriptions: qsTr("Subscriptions"),
                Processed: qsTr("Processed")
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
        if (key === "speedLimit" && value === 0)
            return "Unlimited";
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
        if (clipboardField.text.trim())
            parse(clipboardField.text.trim());
        else
            urlDialog.open();
    }

    function parse(url) {
        manualDismissed = false;
        send({
            "action": "parse",
            "url": url
        });
    }

    function selectedRows() {
        return (state.jobs || []).concat(state.processing || []).filter(function (j) {
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

    function openMediaTools() {
        mediaTools.open();
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
        var rows = activeTab === "Subscriptions" ? (state.subscriptions || []) : activeTab === "Processed" ? (state.processing || []) : (state.jobs || []);
        return rows.filter(function (j) {
            var matches = activeTab === "All" || activeTab === "Subscriptions" || activeTab === "Processed" || (activeTab === "Video" && j.kind === "video") || (activeTab === "Audio" && j.kind === "audio") || (activeTab === "Playlists" && j.kind === "playlist") || (activeTab === "Channels" && j.kind === "channel");
            return matches && (!search || ((j.title || "") + " " + (j.url || "")).toLowerCase().indexOf(search.toLowerCase()) >= 0);
        }).slice().sort(function (a, b) {
            var result = sortKey === "name" ? String(a.title || "").localeCompare(String(b.title || "")) : String(a.created_at || a.id || "").localeCompare(String(b.created_at || b.id || ""));
            return descending ? -result : result;
        });
    }

    objectName: "mainWindow"
    visible: true
    width: 1150
    height: 760
    minimumWidth: 880
    minimumHeight: 600
    title: "VideoDock"
    color: "#fafafa"
    font.family: "Sans Serif"
    font.pixelSize: 14
    palette.window: "#fafafa"
    palette.base: "white"
    palette.button: "#f0f0f0"
    palette.highlight: "#5CB400"
    palette.dark: "#5CB400"
    palette.highlightedText: "white"
    palette.text: "#303030"
    palette.windowText: "#303030"
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
    }

    Backend {
        id: backend

        onSnapshotChanged: {
            try {
                window.state = JSON.parse(snapshot);
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

        visible: false
    }

    Shortcut {
        sequences: [StandardKey.Paste]
        onActivated: window.pasteLink()
    }

    Shortcut {
        sequence: "Ctrl+F"
        onActivated: searchField.forceActiveFocus()
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
            Layout.fillWidth: true
            implicitHeight: 82
            color: "white"

            RowLayout {
                anchors.fill: parent
                anchors.margins: 18
                spacing: 16

                Button {
                    objectName: "pasteButton"
                    text: qsTr("＋  Paste Link")
                    font.pixelSize: 17
                    highlighted: true
                    Layout.preferredWidth: 210
                    Layout.preferredHeight: 44
                    onClicked: window.pasteLink()
                }

                Switch {
                    objectName: "smartModeSwitch"
                    text: qsTr("Smart Mode")
                    checked: window.settings.smartMode === true
                    onToggled: window.setting("smartMode", checked)
                }

                Item {
                    Layout.fillWidth: true
                }

                TextField {
                    id: searchField

                    objectName: "searchField"
                    Layout.preferredWidth: 230
                    placeholderText: qsTr("Search downloads")
                    onTextChanged: window.search = text
                    selectByMouse: true
                }

                ToolButton {
                    text: "◉"
                    font.pixelSize: 23
                    Accessible.name: qsTr("Show Browser")
                    ToolTip.text: qsTr("Show Browser")
                    ToolTip.visible: hovered
                    onClicked: window.browserVisible = !window.browserVisible
                }

                ToolButton {
                    text: "⚙"
                    font.pixelSize: 25
                    Accessible.name: qsTr("Preferences")
                    onClicked: settingsDialog.open()
                }

                ToolButton {
                    text: "☰"
                    font.pixelSize: 25
                    Accessible.name: qsTr("Control Panel")
                    onClicked: controlPanel.open()
                }
            }
        }

        Rectangle {
            Layout.fillWidth: true
            implicitHeight: 54
            color: "#f3f5f1"
            visible: window.settings.smartMode === true

            RowLayout {
                anchors.fill: parent
                anchors.leftMargin: 20
                anchors.rightMargin: 20
                spacing: 12

                Label {
 textFormat: Text.PlainText;
                    text: qsTr("Download")
                    color: "#777"
                }

                ComboBox {
                    model: ["video", "audio"]
                    currentIndex: window.settings.mediaType === "audio" ? 1 : 0
                    onActivated: window.setting("mediaType", currentText)
                    Layout.preferredWidth: 100
                }

                Label {
 textFormat: Text.PlainText;
                    text: qsTr("Quality")
                    color: "#777"
                }

                ComboBox {
                    model: ["Best", "8K", "4K", "1440p", "1080p", "720p", "480p", "360p"]
                    currentIndex: Math.max(0, model.indexOf(window.optionText("quality", window.settings.quality)))
                    onActivated: window.setting("quality", currentText)
                    Layout.preferredWidth: 115
                }

                Label {
 textFormat: Text.PlainText;
                    text: qsTr("Format")
                    color: "#777"
                }

                ComboBox {
                    model: window.settings.mediaType === "audio" ? ["M4A", "MP3", "OGG"] : ["MP4", "MKV"]
                    currentIndex: Math.max(0, model.indexOf(window.optionText("container", window.settings.container)))
                    onActivated: window.setting("container", currentText)
                    Layout.preferredWidth: 100
                }

                Label {
 textFormat: Text.PlainText;
                    text: qsTr("Save To")
                    color: "#777"
                }

                Button {
                    text: window.settings.path || qsTr("Choose Folder…")
                    Layout.fillWidth: true
                    onClicked: folderDialog.open()
                }

                Button {
                    text: qsTr("All Preferences…")
                    onClicked: {
                        settingsDialog.tab = 1;
                        settingsDialog.open();
                    }
                }
            }
        }

        BrowserPane {
            id: browser

            app: window
            visible: window.browserVisible
            Layout.fillWidth: true
            Layout.preferredHeight: window.browserVisible ? Math.max(280, window.height * 0.46) : 0
        }

        Rectangle {
            Layout.fillWidth: true
            implicitHeight: 55
            color: "white"

            RowLayout {
                anchors.fill: parent
                anchors.leftMargin: 16
                anchors.rightMargin: 16
                spacing: 2

                Repeater {
                    model: ["All", "Video", "Audio", "Playlists", "Channels", "Subscriptions", "Processed"]

                    Button {
                        id: filterTab
                        required property string modelData

                        text: window.tabTitle(modelData)
                        flat: true
                        checked: window.activeTab === modelData
                        checkable: true
                        onClicked: {
                            window.activeTab = modelData;
                            window.selectedIds = [];
                        }
                        palette.buttonText: checked ? window.green : "#777"
                        font.bold: checked
                        contentItem: Label {
 textFormat: Text.PlainText;
                            text: filterTab.text
                            color: filterTab.checked ? window.green : "#777"
                            horizontalAlignment: Text.AlignHCenter
                            verticalAlignment: Text.AlignVCenter
                            font: filterTab.font
                        }

                        background: Rectangle {
                            color: "transparent"

                            Rectangle {
                                anchors.bottom: parent.bottom
                                width: parent.width
                                height: 3
                                color: window.green
                                visible: window.activeTab === modelData
                            }
                        }
                    }
                }

                Item {
                    Layout.fillWidth: true
                }

                ComboBox {
                    model: [qsTr("Date Added"), qsTr("Name")]
                    Layout.preferredWidth: 175
                    onActivated: window.sortKey = currentIndex === 0 ? "date" : "name"
                }

                ToolButton {
                    text: window.descending ? "↓" : "↑"
                    onClicked: window.descending = !window.descending
                    Accessible.name: qsTr("Reverse sort direction")
                }
            }
        }

        JobList {
            app: window
            rows: window.listRows()
            Layout.fillWidth: true
            Layout.fillHeight: true
        }

        Rectangle {
            Layout.fillWidth: true
            implicitHeight: noticeLabel.text ? Math.min(90, noticeLabel.implicitHeight + 22) : 0
            color: "#fff2d2"
            visible: implicitHeight > 0

            RowLayout {
                anchors.fill: parent
                anchors.margins: 10

                Label {
 textFormat: Text.PlainText;
                    id: noticeLabel

                    text: window.localNotice || window.state.notice || ""
                    Layout.fillWidth: true
                    wrapMode: Text.Wrap
                    color: "#775614"
                }

                ToolButton {
                    text: "×"
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

        Rectangle {
            Layout.fillWidth: true
            implicitHeight: 32
            color: "#f1f1f1"

            RowLayout {
                anchors.fill: parent
                anchors.leftMargin: 18
                anchors.rightMargin: 18

                Label {
 textFormat: Text.PlainText;
                    text: (window.state.jobs || []).length + qsTr(" items")
                    color: "#888"
                    font.pixelSize: 12
                }

                Item {
                    Layout.fillWidth: true
                }

                Label {
 textFormat: Text.PlainText;
                    text: window.selectedIds.length ? window.selectedIds.length + qsTr(" selected") : "VideoDock"
                    color: "#888"
                    font.pixelSize: 12
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

    MediaToolsDialog {
        id: mediaTools

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

    Dialog {
        id: urlDialog

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

    Dialog {
        id: removeDialog

        title: qsTr("Delete Selected Items")
        modal: true
        anchors.centerIn: parent
        width: 440
        standardButtons: Dialog.Ok | Dialog.Cancel
        onOpened: deleteFiles.checked = false
        onAccepted: {
            window.jobs("remove", window.selectedIds, deleteFiles.checked);
            window.selectedIds = [];
        }

        ColumnLayout {
            width: parent.width

            Label {
 textFormat: Text.PlainText;
                text: qsTr("Remove %1 item(s) from the list?").arg(window.selectedIds.length)
                wrapMode: Text.Wrap
                Layout.fillWidth: true
            }

            CheckBox {
                id: deleteFiles

                text: qsTr("Also delete files from device")
                checked: false
            }
        }
    }

    Popup {
        id: controlPanel

        x: window.width - width - 12
        y: 12
        width: 315
        padding: 20
        modal: false
        closePolicy: Popup.CloseOnEscape | Popup.CloseOnPressOutside

        ColumnLayout {
            width: parent.width
            spacing: 12

            Label {
 textFormat: Text.PlainText;
                text: qsTr("Control Panel")
                font.pixelSize: 20
                font.bold: true
            }

            Label {
 textFormat: Text.PlainText;
                text: (window.state.account || {}).logged_in ? (window.state.account.user_name || "YouTube") : qsTr("YouTube account")
                Layout.fillWidth: true
                elide: Text.ElideRight
            }

            Button {
                text: (window.state.account || {}).logged_in ? qsTr("Log Out") : qsTr("Log In")
                Layout.fillWidth: true
                onClicked: window.send({
                    "action": "account",
                    "command": (window.state.account || {}).logged_in ? "logout" : "login"
                })
            }

            Label {
 textFormat: Text.PlainText;
                text: qsTr("Sign in through Google to access your private playlists.")
                Layout.fillWidth: true
                wrapMode: Text.Wrap
                color: "#777"
            }

            Button {
                text: qsTr('Download "Liked"')
                Layout.fillWidth: true
                onClicked: window.send({
                    "action": "account",
                    "command": "liked"
                })
            }

            Button {
                text: qsTr('Download "Watch Later"')
                Layout.fillWidth: true
                onClicked: window.send({
                    "action": "account",
                    "command": "watch_later"
                })
            }

            Button {
                text: qsTr("Preferences")
                Layout.fillWidth: true
                onClicked: {
                    controlPanel.close();
                    settingsDialog.open();
                }
            }
        }
    }

    Dialog {
        id: about

        title: qsTr("About VideoDock")
        anchors.centerIn: parent
        modal: true
        width: 440
        standardButtons: Dialog.Close

        Label {
 textFormat: Text.PlainText;
            text: qsTr("VideoDock\nRust download engine · Qt interface\nIndependent implementation.")
            padding: 18
            wrapMode: Text.Wrap
        }
    }

    Dialog {
        id: quitDialog

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
 textFormat: Text.PlainText;
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
        Menu {
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

        Menu {
            title: qsTr("Edit")

            MenuItem {
                text: qsTr("Paste Link")
                onTriggered: window.pasteLink()
            }

            Menu {
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

            Menu {
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

        Menu {
            title: qsTr("View")

            Repeater {
                model: ["All", "Video", "Audio", "Playlists", "Channels", "Subscriptions", "Processed"]

                MenuItem {
                    required property string modelData

                    text: window.tabTitle(modelData)
                    checkable: true
                    checked: window.activeTab === modelData
                    onTriggered: window.activeTab = modelData
                }
            }

            MenuSeparator {}

            Menu {
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
        }

        Menu {
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

        Menu {
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
