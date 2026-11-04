pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import QtQuick.Effects

AppDialog {
    id: root

    property var sourceIds: []
    property string errorMessage: ""
    readonly property var effects: [
        {
            id: "remove_vocals",
            title: qsTr("Remove Vocals"),
            description: qsTr("Creates a backing track by removing the vocal part."),
            icon: "ai-remove-vocals.png"
        },
        {
            id: "extract_vocals",
            title: qsTr("Extract Vocals"),
            description: qsTr("Isolates clear vocals by removing the backing track."),
            icon: "ai-extract-vocals.png"
        },
        {
            id: "extract_voice",
            title: qsTr("Extract Voice"),
            description: qsTr("Isolates voice from all background sounds."),
            icon: "ai-extract-voice.png"
        },
        {
            id: "remove_echo",
            title: qsTr("Remove Echo"),
            description: qsTr("Eliminates echo and reverb from voice and vocals."),
            icon: "ai-echo.png"
        }
    ]
    readonly property bool canProcess: (app.state.capabilities || {}).processing === true && sourceIds.length > 0 && sourceIds.every(function (id) {
        const row = (root.app.state.jobs || []).concat(root.app.state.processing || []).find(function (item) {
            return item.id === id;
        });
        return root.validSource(row);
    })

    function validSource(row) {
        if (!row)
            return false;
        if (Array.isArray(row.items))
            return row.items.length > 0 && row.items.every(validSource);
        return ["video", "audio"].indexOf(row.kind) >= 0 && row.state === "complete" && typeof row.path === "string" && row.path.length > 0;
    }

    function chooseEffect(effectId) {
        if (!canProcess || !effects.some(function (effect) {
            return effect.id === effectId;
        }))
            return;
        const result = app.send({
            action: "process",
            ids: sourceIds.slice(),
            effect: effectId
        });
        if (result.error)
            errorMessage = String(result.error);
        else
            close();
    }

    title: qsTr("Media Tools")
    objectName: "mediaToolsDialog"
    modal: true
    anchors.centerIn: parent
    width: Math.min(548, parent.width - 40)
    height: Math.min(384, app.height - 90)
    padding: 0
    header: null
    footer: null
    closePolicy: Popup.CloseOnEscape
    Overlay.modal: Rectangle {
        color: Qt.rgba(0, 0, 0, 0.6)
    }
    background: Rectangle {
        color: root.app.theme.dark ? "#333333" : "#f6f6f6"
        radius: 2
    }
    onOpened: {
        sourceIds = Array.isArray(app.selectedIds) ? app.selectedIds.slice() : [];
        errorMessage = "";
    }

    contentItem: ColumnLayout {
        spacing: 0
        Rectangle {
            id: header
            objectName: "mediaToolsHeader"
            Layout.fillWidth: true
            Layout.preferredHeight: 56
            color: root.app.theme.window
            z: 1
            layer.enabled: true
            layer.effect: MultiEffect {
                shadowEnabled: true
                shadowBlur: 0.4
                shadowColor: Qt.rgba(0, 0, 0, 0.1)
                shadowVerticalOffset: 4
                shadowHorizontalOffset: 0
            }
            Label {
                x: 24
                anchors.verticalCenter: parent.verticalCenter
                width: parent.width - 80
                text: root.title
                font.pixelSize: 18
                elide: Text.ElideRight
            }
            ToolButton {
                id: closeButton
                objectName: "mediaToolsCloseButton"
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
                ToolTip.text: qsTr("Close")
                ToolTip.visible: hovered
                background: Rectangle {
                    radius: 12
                    color: closeButton.hovered || closeButton.visualFocus ? root.app.theme.hover : "transparent"
                }
                onClicked: root.close()
            }
        }

        Label {
            objectName: "processingErrorText"
            text: root.errorMessage
            visible: text.length > 0
            textFormat: Text.PlainText
            font.pixelSize: 14
            color: root.app.theme.error
            wrapMode: Text.Wrap
            Layout.fillWidth: true
            Layout.leftMargin: 24
            Layout.rightMargin: 24
            Layout.topMargin: 12
        }

        ScrollView {
            id: scroll
            Layout.fillWidth: true
            Layout.fillHeight: true
            contentWidth: availableWidth
            contentHeight: grid.implicitHeight + 48
            clip: true
            GridLayout {
                id: grid
                objectName: "mediaToolsGrid"
                x: 24
                y: 24
                width: scroll.availableWidth - 48
                columns: 2
                columnSpacing: 8
                rowSpacing: 8
                readonly property real cellWidth: (width + columnSpacing) / columns
                readonly property int cellHeight: 136 + rowSpacing
                Repeater {
                    model: root.effects
                    Button {
                        id: card
                        required property var modelData
                        objectName: "effectCard_" + modelData.id
                        Layout.fillWidth: true
                        Layout.preferredWidth: grid.cellWidth - grid.columnSpacing
                        Layout.preferredHeight: 136
                        padding: 16
                        enabled: root.canProcess
                        Accessible.name: modelData.title + ". " + modelData.description
                        background: Rectangle {
                            radius: 2
                            color: card.hovered || card.visualFocus ? (root.app.theme.dark ? "#383838" : Qt.rgba(233 / 255, 233 / 255, 233 / 255, 0.6)) : root.app.theme.window
                        }
                        onClicked: root.chooseEffect(modelData.id)
                        contentItem: ColumnLayout {
                            spacing: 0
                            Rectangle {
                                width: 32
                                height: 32
                                color: root.app.theme.dark ? Qt.rgba(0, 0, 0, 0.1) : Qt.rgba(246 / 255, 246 / 255, 246 / 255, 0.84)
                                Image {
                                    anchors.centerIn: parent
                                    width: 20
                                    height: 20
                                    source: root.app.asset(card.modelData.icon, "icons/audio.svg")
                                }
                            }
                            Label {
                                Layout.fillWidth: true
                                Layout.topMargin: 8
                                text: card.modelData.title
                                font.pixelSize: 14
                                color: root.app.theme.text
                                elide: Text.ElideRight
                            }
                            Label {
                                Layout.fillWidth: true
                                Layout.topMargin: 4
                                text: card.modelData.description
                                font.pixelSize: 14
                                lineHeight: 20
                                lineHeightMode: Text.FixedHeight
                                color: root.app.theme.secondary
                                wrapMode: Text.Wrap
                                elide: Text.ElideRight
                                maximumLineCount: 2
                            }
                            Item {
                                Layout.fillHeight: true
                            }
                        }
                    }
                }
            }
        }
    }
}
