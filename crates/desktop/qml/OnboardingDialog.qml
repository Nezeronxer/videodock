pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

AppDialog {
    id: root
    objectName: "onboardingDialog"

    property int step: 0
    property var steps: [
        { "mark": "plus", "caption": qsTr("Easy download start"), "title": qsTr("Add a video link to the app"), "body": qsTr("1. Copy the video URL from your browser.") + "<br>" + qsTr("2. Click the Paste Link button within the app.") },
        { "mark": "search", "caption": qsTr("Quick video search"), "title": qsTr("Find videos right in the app"), "body": qsTr("Select the service you want to download from.") + "<br>" + qsTr("Follow the video link and click Download.") },
        { "mark": "check", "caption": qsTr("Default settings setup"), "title": qsTr("Specify your preferences"), "body": qsTr("Enable Smart Mode to set quality, format, device, etc.") + "<br>" + qsTr("Preferences will be automatically applied to all links.") },
        { "imageStep": 5, "mark": "subscribe", "caption": qsTr("Subscriptions addition"), "title": qsTr("Subscribe to channels and playlists"), "body": qsTr("Add links to favorite YouTube creators and playlists.") + "<br>" + qsTr("Grab all existing videos, auto-download new content.") }
    ]

    title: qsTr("Getting Started")
    modal: true
    dim: false
    anchors.centerIn: parent
    width: parent.width
    height: parent.height
    padding: 0
    header: null
    footer: null
    closePolicy: Popup.NoAutoClose
    background: Rectangle { color: root.app.theme.canvas }
    onOpened: step = 0

    function finish() {
        app.send({ "action": "app", "command": "onboarding_complete" });
        close();
    }

    contentItem: Item {
        ColumnLayout {
            anchors.horizontalCenter: parent.horizontalCenter
            y: Math.max(16, (parent.height - height) / 2 + 40)
            width: Math.min(611, parent.width - 32)
            spacing: 48

            ColumnLayout {
                Layout.fillWidth: true
                spacing: 4
                Label {
                    text: root.steps[root.step].caption
                    color: root.app.theme.secondary
                    font.pixelSize: 14
                    Layout.fillWidth: true
                    horizontalAlignment: Text.AlignHCenter
                }
                Label {
                    textFormat: Text.PlainText
                    text: root.steps[root.step].title
                    font.pixelSize: 28
                    wrapMode: Text.Wrap
                    Layout.fillWidth: true
                    horizontalAlignment: Text.AlignHCenter
                }
            }

            Rectangle {
                Layout.fillWidth: true
                implicitHeight: illustration.height + actions.implicitHeight + 40
                color: root.app.theme.window

                Rectangle {
                    id: illustration
                    x: 16
                    y: 16
                    width: parent.width - 32
                    height: Math.min(265, Math.max(120, root.height - 310))
                    color: root.app.theme.hover

                    Rectangle {
                        x: 40
                        y: 27
                        width: parent.width - 80
                        height: 20
                        radius: 7
                        color: root.app.theme.panel
                    }
                    Rectangle {
                        x: 40
                        y: 40
                        width: parent.width - 80
                        height: parent.height - 40
                        color: root.app.theme.list
                    }
                    ToolButton {
                        anchors.centerIn: parent
                        width: 112
                        height: 80
                        enabled: false
                        display: AbstractButton.IconOnly
                        icon.source: "icons/" + root.steps[root.step].mark + ".svg"
                        icon.width: 56
                        icon.height: 56
                        icon.color: root.app.green
                        background: Rectangle {
                            radius: 4
                            color: root.app.theme.selected
                        }
                    }
                    AnimatedImage {
                        anchors.fill: parent
                        source: root.app.asset("onboarding-step-" + (root.steps[root.step].imageStep || (root.step + 1)) + ".gif", "")
                        visible: source.toString().length > 0
                        playing: visible && root.visible
                        fillMode: Image.Stretch
                    }
                }

                RowLayout {
                    id: actions
                    anchors.top: illustration.bottom
                    anchors.left: parent.left
                    anchors.right: parent.right
                    anchors.margins: 16
                    anchors.topMargin: 12
                    spacing: 8

                    Label {
                        textFormat: Text.RichText
                        text: root.steps[root.step].body
                        lineHeightMode: Text.FixedHeight
                        lineHeight: 20
                        color: root.app.theme.secondary
                        font.pixelSize: 14
                        wrapMode: Text.Wrap
                        Layout.fillWidth: true
                        Layout.rightMargin: 10
                    }
                    Button {
                        objectName: "onboardingSecondary"
                        text: root.step === 0 ? qsTr("Skip") : qsTr("Back")
                        flat: true
                        palette.windowText: root.app.green
                        Layout.preferredWidth: 100
                        Layout.preferredHeight: 40
                        onClicked: root.step === 0 ? root.finish() : root.step--
                    }
                    Button {
                        objectName: "onboardingPrimary"
                        text: root.step === root.steps.length - 1 ? qsTr("Finish") : qsTr("Next")
                        highlighted: true
                        palette.dark: root.app.green
                        palette.highlightedText: root.app.theme.accentText
                        Layout.preferredWidth: 100
                        Layout.preferredHeight: 40
                        onClicked: root.step === root.steps.length - 1 ? root.finish() : root.step++
                    }
                }
            }
        }
    }
}
