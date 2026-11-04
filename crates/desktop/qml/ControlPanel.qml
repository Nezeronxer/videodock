pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import QtQuick.Effects

Popup {
    id: root

    required property var app
    signal preferencesRequested
    property real slideOffset: 0
    property string errorMessage: ""
    readonly property bool loggedIn: (app.state.account || {}).logged_in === true
    readonly property string username: sanitizeText((app.state.account || {}).user_name).replace(/\s+/g, " ").trim()
    readonly property string avatarUrl: String((app.state.account || {}).avatar_url || "")
    property bool avatarFailed: false
    readonly property bool hasAvatar: loggedIn && avatarUrl.indexOf("https://") === 0 && !avatarFailed
    readonly property var notification: app.state.notification || null
    onAvatarUrlChanged: avatarFailed = false
    onLoggedInChanged: avatarFailed = false

    objectName: "controlPanel"
    palette: app.palette
    width: 372
    implicitHeight: Math.max(602, 120 + bodyColumn.implicitHeight)
    height: Math.max(implicitHeight, parent.height - 24)
    x: parent.width - width - 12 + slideOffset
    y: 12
    padding: 0
    modal: true
    dim: false
    focus: true
    closePolicy: Popup.CloseOnEscape | Popup.CloseOnPressOutside
    onOpened: errorMessage = ""

    function sanitizeText(value) {
        return value === null || value === undefined ? "" : String(value).replace(/[\u0000-\u0008\u000b\u000c\u000e-\u001f\u007f\u202a-\u202e\u2066-\u2069]/g, "").slice(0, 1500);
    }
    function accountAction(command) {
        if (["login", "logout", "liked", "watch_later"].indexOf(command) < 0)
            return;
        const result = app.send({
            action: "account",
            command: command
        });
        if (result.error)
            errorMessage = sanitizeText(result.error);
        else
            close();
    }
    function showPreferences() {
        close();
        preferencesRequested();
    }

    enter: Transition {
        ParallelAnimation {
            NumberAnimation {
                target: root
                property: "slideOffset"
                from: 20
                to: 0
                duration: 200
                easing.type: Easing.OutCubic
            }
            NumberAnimation {
                target: root
                property: "opacity"
                from: 0
                to: 1
                duration: 200
                easing.type: Easing.OutCubic
            }
        }
    }
    exit: Transition {
        ParallelAnimation {
            NumberAnimation {
                target: root
                property: "slideOffset"
                from: 0
                to: 20
                duration: 200
                easing.type: Easing.InCubic
            }
            NumberAnimation {
                target: root
                property: "opacity"
                from: 1
                to: 0
                duration: 250
                easing.type: Easing.InCubic
            }
        }
    }
    background: Rectangle {
        color: root.app.theme.panel
        opacity: 0.9
        radius: 6
        border.color: root.app.theme.border
        border.width: 1
        layer.enabled: true
        layer.effect: MultiEffect {
            shadowEnabled: true
            shadowColor: Qt.rgba(0, 0, 0, 0.1)
            shadowBlur: 0.4
            shadowVerticalOffset: 2
        }
    }

    component PanelAction: Button {
        id: actionButton
        required property string imageName
        property string fallbackImage: ""
        Layout.fillWidth: true
        implicitWidth: 348
        implicitHeight: 40
        horizontalPadding: 12
        contentItem: RowLayout {
            spacing: 8
            Image {
                Layout.preferredWidth: 20
                Layout.preferredHeight: 20
                source: root.app.asset(actionButton.imageName, actionButton.fallbackImage)
                layer.enabled: root.app.theme.dark
                layer.effect: MultiEffect {
                    colorization: 1
                    colorizationColor: root.app.theme.text
                }
            }
            Label {
                Layout.fillWidth: true
                text: actionButton.text
                textFormat: Text.PlainText
                font.pixelSize: 14
                color: root.app.theme.text
                elide: Text.ElideRight
            }
        }
        background: Rectangle {
            radius: 4
            color: actionButton.down ? root.app.theme.selected : actionButton.hovered || actionButton.visualFocus ? (root.app.theme.dark ? "#505050" : "#e9e9e9") : root.app.theme.dark ? "#4e4e4e" : "#ffffff"
        }
    }

    contentItem: Item {
        ToolButton {
            id: closeButton
            objectName: "controlPanelCloseButton"
            anchors.right: parent.right
            anchors.rightMargin: 8
            y: 16
            width: 28
            height: 28
            z: 2
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
        ToolButton {
            id: headerPreferences
            objectName: "controlPanelHeaderPreferencesButton"
            x: 8
            y: 16
            width: 28
            height: 28
            z: 2
            visible: body.contentItem.contentY >= preferences.height
            display: AbstractButton.IconOnly
            icon.source: root.app.asset("settings.png", "icons/settings.svg")
            icon.width: 20
            icon.height: 20
            icon.color: root.app.theme.text
            Accessible.name: qsTr("Preferences")
            background: Rectangle {
                radius: 14
                color: headerPreferences.hovered || headerPreferences.visualFocus ? root.app.theme.hover : "transparent"
            }
            onClicked: root.showPreferences()
        }
        Button {
            id: auth
            property real hitWidth: 48
            readonly property real labelWidth: Math.min(parent.width - 48, Math.ceil(Math.max(48, accountLabel.implicitWidth) / 2) * 2)
            onLabelWidthChanged: hitWidth = Math.max(hitWidth, labelWidth)
            objectName: "controlPanelAuthButton"
            anchors.horizontalCenter: parent.horizontalCenter
            y: 20
            width: hitWidth
            height: 80
            padding: 0
            Accessible.name: accountLabel.text
            background: Item {}
            onClicked: root.accountAction(root.loggedIn ? "logout" : "login")
            contentItem: ColumnLayout {
                spacing: 4
                Item {
                    Layout.alignment: Qt.AlignHCenter
                    implicitWidth: 48
                    implicitHeight: 48
                    Image {
                        id: avatar
                        anchors.fill: parent
                        source: root.hasAvatar ? root.avatarUrl : root.app.asset("youtube-primary.png", "icons/service-youtube.svg")
                        fillMode: Image.PreserveAspectCrop
                        layer.enabled: root.hasAvatar
                        layer.effect: MultiEffect {
                            maskEnabled: true
                            maskSource: avatarMask
                        }
                        onStatusChanged: if (status === Image.Error && root.hasAvatar)
                            root.avatarFailed = true
                    }
                    Rectangle {
                        id: avatarMask
                        width: 48
                        height: 48
                        radius: 10
                        layer.enabled: true
                        visible: false
                    }
                    Image {
                        visible: root.hasAvatar
                        anchors.right: parent.right
                        anchors.bottom: parent.bottom
                        width: 16
                        height: 16
                        source: root.app.asset("youtube-secondary.png", "icons/service-youtube.svg")
                    }
                }
                Rectangle {
                    Layout.alignment: Qt.AlignHCenter
                    Layout.preferredWidth: auth.labelWidth
                    implicitHeight: 28
                    radius: 14
                    color: root.loggedIn && !auth.hovered ? "transparent" : root.app.theme.dark ? Qt.rgba(1, 1, 1, auth.hovered ? 0.05 : 0.1) : auth.hovered ? Qt.rgba(1, 1, 1, 0.4) : "#ffffff"
                    Label {
                        id: accountLabel
                        objectName: "controlPanelAccountLabel"
                        anchors.fill: parent
                        leftPadding: 12
                        rightPadding: 12
                        text: root.loggedIn ? auth.hovered ? qsTr("Log Out") : root.username || "YouTube" : qsTr("Log In")
                        textFormat: Text.PlainText
                        font.pixelSize: 14
                        verticalAlignment: Text.AlignVCenter
                        horizontalAlignment: Text.AlignHCenter
                        elide: Text.ElideRight
                    }
                }
            }
        }

        ScrollView {
            id: body
            objectName: "controlPanelBody"
            y: 120
            width: parent.width
            height: parent.height - y
            contentWidth: availableWidth
            contentHeight: bodyColumn.implicitHeight
            clip: true
            ColumnLayout {
                id: bodyColumn
                x: 12
                width: body.availableWidth - 24
                spacing: 12
                PanelAction {
                    id: preferences
                    objectName: "controlPanelPreferencesButton"
                    imageName: "settings.png"
                    fallbackImage: "icons/settings.svg"
                    text: qsTr("Preferences")
                    onClicked: root.showPreferences()
                }
                Rectangle {
                    Layout.fillWidth: true
                    implicitHeight: playlistActions.implicitHeight
                    radius: 4
                    color: root.app.theme.dark ? "#4e4e4e" : "#ffffff"
                    ColumnLayout {
                        id: playlistActions
                        width: parent.width
                        spacing: 0
                        PanelAction {
                            objectName: "controlPanelLikedButton"
                            imageName: "icon-heart.png"
                            fallbackImage: "icons/audio.svg"
                            text: qsTr('Download "Liked"')
                            onClicked: root.accountAction("liked")
                        }
                        PanelAction {
                            objectName: "controlPanelWatchLaterButton"
                            imageName: "clock.png"
                            fallbackImage: "icons/subscribe.svg"
                            text: qsTr('Download "Watch Later"')
                            onClicked: root.accountAction("watch_later")
                        }
                    }
                }
                // No news feed is supplied by the current Engine.
                Item {
                    objectName: "controlPanelNewsSlot"
                    Layout.fillWidth: true
                    visible: false
                    implicitHeight: 0
                }
                Rectangle {
                    Layout.fillWidth: true
                    visible: root.notification !== null
                    implicitHeight: notificationContent.implicitHeight + 24
                    radius: 4
                    color: root.app.theme.dark ? "#333333" : "#ffffff"
                    ColumnLayout {
                        id: notificationContent
                        x: 12
                        y: 12
                        width: parent.width - 24
                        spacing: 8
                        Label {
                            Layout.fillWidth: true
                            text: root.notification ? root.sanitizeText(root.notification.title) : ""
                            textFormat: Text.PlainText
                            font.pixelSize: 14
                            wrapMode: Text.Wrap
                            maximumLineCount: 2
                            elide: Text.ElideRight
                        }
                        Label {
                            objectName: "controlPanelNotificationBody"
                            Layout.fillWidth: true
                            text: root.notification ? root.sanitizeText(root.notification.body) : ""
                            textFormat: Text.PlainText
                            font.pixelSize: 14
                            color: root.app.theme.secondary
                            wrapMode: Text.Wrap
                            maximumLineCount: 6
                            elide: Text.ElideRight
                        }
                    }
                }
                Label {
                    Layout.fillWidth: true
                    text: root.errorMessage
                    visible: text.length > 0
                    textFormat: Text.PlainText
                    color: root.app.theme.error
                    wrapMode: Text.Wrap
                    maximumLineCount: 4
                    elide: Text.ElideRight
                }
            }
        }
    }
}
