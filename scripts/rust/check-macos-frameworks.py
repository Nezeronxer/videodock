"""Regression: nested Qt helpers must resolve the same bundled framework tree."""
import importlib.util
from pathlib import Path
import tempfile

spec = importlib.util.spec_from_file_location('repair', Path(__file__).with_name('repair-macos-frameworks.py'))
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)
with tempfile.TemporaryDirectory() as directory:
    frameworks = Path(directory) / 'VideoDock.app/Contents/Frameworks'
    library = frameworks / 'QtWebChannel.framework/Versions/A/QtWebChannel'
    library.parent.mkdir(parents=True)
    library.touch()
    for dependency in (
        '@executable_path/../Frameworks/QtWebChannel.framework/Versions/A/QtWebChannel',
        '@rpath/QtWebChannel.framework/Versions/A/QtWebChannel',
        '/usr/local/opt/qtwebchannel/lib/QtWebChannel.framework/Versions/A/QtWebChannel',
        '/opt/homebrew/opt/qtwebchannel/lib/QtWebChannel.framework/Versions/A/QtWebChannel',
    ):
        assert module.bundled_dependency(dependency, frameworks) == library
    assert module.bundled_dependency('/usr/lib/libSystem.B.dylib', frameworks) is None
    assert module.bundled_dependency('@loader_path/liblocal.dylib', frameworks) is None
    try:
        module.bundled_dependency('@rpath/QtMissing.framework/Versions/A/QtMissing', frameworks)
    except RuntimeError:
        pass
    else:
        raise AssertionError('Missing bundled framework was accepted')
print('macOS framework dependency regression: passed')
