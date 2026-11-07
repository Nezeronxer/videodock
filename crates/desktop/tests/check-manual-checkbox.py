"""Exercise the actual batch checkbox binding during delegate role initialization."""
import os
from pathlib import Path
import re
import subprocess
import tempfile


project = Path(__file__).resolve().parents[3]
source = (project / 'crates/desktop/qml/ManualDialog.qml').read_text()
binding = re.search(r'CheckBox\s*\{\s*checked: ([^\n]+)', source)
assert binding, 'Batch checkbox binding not found'
qml = '''pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Controls
ApplicationWindow {
    id: root
    width: 400; height: 300
    visible: true
    property var selectedItems: ["one"]
    property bool userSelection: false
    function check(condition, message) {
        if (!condition) { console.error("FAIL: " + message); Qt.quit(); }
        return condition;
    }
    ListView {
        id: list
        anchors.fill: parent
        model: [{id:"one"}]
        delegate: ItemDelegate {
            required property var modelData
            width: ListView.view.width
            height: 56
            contentItem: CheckBox {
                objectName: "selection"
                checked: BINDING
            }
        }
    }
    Timer {
        interval: 50; running: true; repeat: true
        property int step: 0
        onTriggered: {
            var row = list.itemAtIndex(0);
            if (!root.check(!!row, "Delegate missing")) return;
            var checkbox = row.contentItem;
            if (step === 0) {
                if (!root.check(checkbox.checked, "Initial selection missing")) return;
                row.modelData = undefined;
                root.selectedItems = [];
            } else if (step === 1) {
                if (!root.check(!checkbox.checked, "Uninitialized item stayed selected")) return;
                row.modelData = {id:"one"};
                root.selectedItems = ["one"];
            } else {
                if (!root.check(checkbox.checked, "Selection did not recover after role initialization")) return;
                console.log("PASS: batch checkbox initialization and selection recovery");
                Qt.quit();
            }
            ++step;
        }
    }
}
'''.replace('BINDING', binding[1])
with tempfile.TemporaryDirectory(prefix='videodock-checkbox-') as temporary:
    path = Path(temporary) / 'Checkbox.qml'
    path.write_text(qml)
    result = subprocess.run(['qml', str(path)], env={**os.environ, 'QT_QPA_PLATFORM': 'offscreen',
                            'QT_QUICK_CONTROLS_STYLE': 'Basic', 'QT_QUICK_BACKEND': 'software'}, capture_output=True, text=True, timeout=15)
output = result.stdout + result.stderr
assert result.returncode == 0, output
assert 'TypeError' not in output and 'Error:' not in output and 'FAIL:' not in output, output
assert 'PASS: batch checkbox initialization and selection recovery' in output, output
print('PASS: actual batch checkbox binding tolerates unset roles and recovers selection')
