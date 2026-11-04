import QtQuick

QtObject {
    property string mode: "dark"
    readonly property bool dark: mode === "dark" || (mode === "system" && Qt.styleHints.colorScheme === Qt.Dark)
    readonly property color accent: dark ? "#5fb21f" : "#5cb400"
    readonly property color window: dark ? "#292929" : "#ffffff"
    readonly property color canvas: dark ? "#292929" : "#f6f6f6"
    readonly property color toolbar: dark ? "#333333" : "#ffffff"
    readonly property color popup: dark ? "#383838" : "#ffffff"
    readonly property color panel: dark ? "#3c3c3c" : "#eaeaea"
    readonly property color list: dark ? "#262626" : "#fbf9fb"
    readonly property color hover: dark ? "#3b3b3b" : "#f6f6f6"
    readonly property color selected: dark ? "#383c40" : "#fdf8e6"
    readonly property color input: dark ? "#292929" : "#ffffff"
    readonly property color button: dark ? "#474747" : "#f0f0f0"
    readonly property color border: dark ? "#3b3b3b" : "#e9e9e9"
    readonly property color text: dark ? "#eeeeee" : "#000000"
    readonly property color secondary: dark ? "#828282" : "#8c8c8c"
    readonly property color disabled: dark ? "#858585" : "#8c8c8c"
    readonly property color placeholder: dark ? "#575757" : "#e4e4e4"
    readonly property color placeholderBase: dark ? "#424242" : "#d7d6d6"
    readonly property color error: "#fc461e"
    readonly property color notice: dark ? "#443a22" : "#fff2d2"
    readonly property color noticeText: dark ? "#f5d48b" : "#775614"
    readonly property color accentText: "#ffffff"
}
