import QtQuick
import QtQuick.Controls

Dialog {
    id: dialog
    required property var app
    palette: app.palette
    Overlay.modal: Rectangle { color: Qt.rgba(0, 0, 0, 0.6) }
    background: Rectangle {
        color: dialog.app.theme.window
        border.color: dialog.app.theme.border
        radius: 2
    }
    footer: DialogButtonBox {
        // Dialog connects standard buttons itself; forwarding fires handlers twice.
        standardButtons: dialog.standardButtons
        alignment: Qt.AlignRight
        visible: count > 0
        delegate: Button {
            width: Math.max(100, implicitWidth)
            palette.dark: dialog.app.green
        }
    }
}
