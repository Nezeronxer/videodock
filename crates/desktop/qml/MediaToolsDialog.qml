pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

Dialog {
    id: root

    required property var app
    property string selectedEffect: "remove_vocals"
    property var effects: [
        {
            "id": "remove_vocals",
            "title": qsTr("Remove Vocals"),
            "description": qsTr("Create a backing track by removing vocals."),
            "available": true
        },
        {
            "id": "extract_vocals",
            "title": qsTr("Extract Vocals"),
            "description": qsTr("Separate vocals from the backing track."),
            "available": true
        },
        {
            "id": "extract_voice",
            "title": qsTr("Extract Voice"),
            "description": qsTr("Isolate voice from background sounds."),
            "available": true
        },
        {
            "id": "remove_echo",
            "title": qsTr("Remove Echo"),
            "description": qsTr("Reduce echo and reverberation."),
            "available": true
        },
        {
            "id": "burn_captions",
            "title": qsTr("Burn In Captions"),
            "available": false
        },
        {
            "id": "transcribe",
            "title": qsTr("Transcribe Speech"),
            "available": false
        },
        {
            "id": "remove_music",
            "title": qsTr("Remove Background Music"),
            "available": false
        },
        {
            "id": "remove_noise",
            "title": qsTr("Remove Noise"),
            "available": false
        },
        {
            "id": "instruments",
            "title": qsTr("Extract Instruments"),
            "available": false
        },
        {
            "id": "karaoke",
            "title": qsTr("Make Karaoke"),
            "available": false
        },
        {
            "id": "highlights",
            "title": qsTr("Create Highlights"),
            "available": false
        },
        {
            "id": "loudness",
            "title": qsTr("Balance Loudness"),
            "available": false
        },
        {
            "id": "upscale_audio",
            "title": qsTr("Upscale Audio"),
            "available": false
        },
        {
            "id": "voice",
            "title": qsTr("Change Voice"),
            "available": false
        },
        {
            "id": "lead_vocal",
            "title": qsTr("Extract Lead Vocal"),
            "available": false
        },
        {
            "id": "back_vocal",
            "title": qsTr("Extract Back Vocal"),
            "available": false
        },
        {
            "id": "reframe",
            "title": qsTr("Reframe Video"),
            "available": false
        },
        {
            "id": "upscale_video",
            "title": qsTr("Upscale Video"),
            "available": false
        }
    ]

    title: qsTr("Media Tools")
    objectName: "mediaToolsDialog"
    modal: true
    anchors.centerIn: parent
    width: Math.min(820, parent.width - 50)
    height: Math.min(620, parent.height - 50)

    contentItem: ColumnLayout {
        Label {
 textFormat: Text.PlainText;
            text: root.app.localNotice
            visible: text.length > 0
            color: "#b94332"
            wrapMode: Text.Wrap
            Layout.fillWidth: true
        }
        Label {
 textFormat: Text.PlainText;
            text: qsTr("Choose a tool for %1 selected download(s)").arg(root.app.selectedIds.length)
            font.pixelSize: 18
            Layout.fillWidth: true
            wrapMode: Text.Wrap
        }

        Label {
 textFormat: Text.PlainText;
            text: qsTr("Processing runs locally. Availability depends on the source audio and installed models.")
            color: "#888"
            wrapMode: Text.Wrap
            Layout.fillWidth: true
        }

        ScrollView {
            Layout.fillWidth: true
            Layout.fillHeight: true
            contentWidth: availableWidth

            GridLayout {
                width: parent.width
                columns: 3
                columnSpacing: 10
                rowSpacing: 10

                Repeater {
                    model: root.effects

                    Button {
                        id: effectButton
                        required property var modelData

                        Layout.fillWidth: true
                        Layout.preferredHeight: 95
                        enabled: modelData.available
                        highlighted: root.selectedEffect === modelData.id
                        onClicked: root.selectedEffect = modelData.id

                        contentItem: ColumnLayout {
                            Label {
 textFormat: Text.PlainText;
                                text: modelData.title
                                font.bold: true
                                color: effectButton.highlighted ? "white" : effectButton.modelData.available ? "#444" : "#999"
                                wrapMode: Text.Wrap
                                Layout.fillWidth: true
                                horizontalAlignment: Text.AlignHCenter
                            }

                            Label {
 textFormat: Text.PlainText;
                                text: modelData.available ? modelData.description : qsTr("Not available yet")
                                color: effectButton.highlighted ? "#efffe8" : "#999"
                                font.pixelSize: 11
                                wrapMode: Text.Wrap
                                Layout.fillWidth: true
                                horizontalAlignment: Text.AlignHCenter
                            }
                        }
                    }
                }
            }
        }

        RowLayout {
            Item {
                Layout.fillWidth: true
            }

            Button {
                text: qsTr("Cancel")
                onClicked: root.close()
            }

            Button {
                text: qsTr("Process")
                highlighted: true
                enabled: root.app.selectedIds.length > 0 && (root.app.state.capabilities || {}).processing !== false
                onClicked: {
                    var result = root.app.send({
                        "action": "process",
                        "ids": root.app.selectedIds,
                        "effect": root.selectedEffect
                    });
                    if (!result.error)
                        root.close();
                }
            }
        }
    }
}
