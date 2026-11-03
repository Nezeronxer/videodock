pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

Dialog {
    id: root

    required property var app
    property int step: 0
    property var steps: [
        {
            "mark": "＋",
            "title": qsTr("Add a video link"),
            "body": qsTr("Copy a video or song URL from your browser and click Paste Link.")
        },
        {
            "mark": "⌕",
            "title": qsTr("Find videos in the app"),
            "body": qsTr("Open the browser, choose a service and search. Click Download on a video page.")
        },
        {
            "mark": "✓",
            "title": qsTr("Choose your preferences"),
            "body": qsTr("Enable Smart Mode to use the same quality, format, subtitles and output folder for new links.")
        },
        {
            "mark": "♫",
            "title": qsTr("Try Media Tools"),
            "body": qsTr("Select a completed download to extract vocals, isolate voice or remove echo.")
        },
        {
            "mark": "↻",
            "title": qsTr("Subscribe to channels"),
            "body": qsTr("Paste a channel or playlist link and choose Subscribe. Download existing videos or only new releases.")
        }
    ]

    title: qsTr("Getting Started")
    modal: true
    anchors.centerIn: parent
    width: Math.min(580, parent.width - 50)
    height: 430
    onOpened: step = 0

    contentItem: ColumnLayout {
        spacing: 18

        Rectangle {
            Layout.alignment: Qt.AlignHCenter
            Layout.preferredWidth: 115
            Layout.preferredHeight: 115
            radius: 58
            color: "#edf5e8"

            Label {
 textFormat: Text.PlainText;
                text: root.steps[root.step].mark
                anchors.centerIn: parent
                font.pixelSize: 66
                color: root.app.green
            }
        }

        Label {
 textFormat: Text.PlainText;
            text: root.steps[root.step].title
            Layout.fillWidth: true
            horizontalAlignment: Text.AlignHCenter
            font.pixelSize: 25
            font.bold: true
        }

        Label {
 textFormat: Text.PlainText;
            text: root.steps[root.step].body
            Layout.fillWidth: true
            horizontalAlignment: Text.AlignHCenter
            wrapMode: Text.Wrap
            color: "#888"
            lineHeight: 1.5
        }

        Item {
            Layout.fillHeight: true
        }

        RowLayout {
            Layout.alignment: Qt.AlignHCenter

            Repeater {
                model: root.steps.length

                Rectangle {
                    required property int index

                    width: 7
                    height: 7
                    radius: 4
                    color: index === root.step ? root.app.green : "#ddd"
                }
            }
        }

        RowLayout {
            Button {
                text: qsTr("Skip")
                flat: true
                onClicked: {
                    root.app.send({
                        "action": "app",
                        "command": "onboarding_complete"
                    });
                    root.close();
                }
            }

            Item {
                Layout.fillWidth: true
            }

            Button {
                text: qsTr("Back")
                enabled: root.step > 0
                onClicked: root.step--
            }

            Button {
                text: root.step === root.steps.length - 1 ? qsTr("Finish") : qsTr("Next")
                highlighted: true
                onClicked: {
                    if (root.step < root.steps.length - 1) {
                        root.step++;
                    } else {
                        root.app.send({
                            "action": "app",
                            "command": "onboarding_complete"
                        });
                        root.close();
                    }
                }
            }
        }
    }
}
