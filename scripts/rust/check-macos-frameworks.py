"""Regression: nested Qt helpers must resolve the same bundled framework tree."""
import importlib.util
from pathlib import Path
import plistlib
import subprocess
import sys
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

if sys.platform == 'darwin':
    with tempfile.TemporaryDirectory(prefix='videodock-signing-contract-') as directory:
        root = Path(directory)
        app = root / 'Fixture.app'
        frameworks = app / 'Contents/Frameworks'
        plugins = app / 'Contents/PlugIns'
        executable = app / 'Contents/MacOS/Fixture'
        for folder in (frameworks, plugins, executable.parent):
            folder.mkdir(parents=True)
        source = root / 'fixture.c'
        source.write_text('''#ifdef BUILD_MAIN
extern int fixture_value(void);
int main(void) { return fixture_value(); }
#elif defined(BUILD_PLUGIN)
extern int fixture_value(void);
int plugin_value(void) { return fixture_value(); }
#else
int fixture_value(void) { return 0; }
#endif
''')
        library = frameworks / 'libFixture.dylib'
        plugin = plugins / 'libVendor.dylib'
        subprocess.run(['clang', '-dynamiclib', '-Wl,-headerpad_max_install_names',
                        '-install_name', '@rpath/libFixture.dylib', str(source), '-o', str(library)], check=True)
        subprocess.run(['clang', '-DBUILD_MAIN', '-Wl,-headerpad_max_install_names',
                        '-Wl,-rpath,@executable_path/../Frameworks', str(source),
                        '-L' + str(frameworks), '-lFixture', '-o', str(executable)], check=True)
        subprocess.run(['clang', '-DBUILD_PLUGIN', '-dynamiclib', '-Wl,-headerpad_max_install_names',
                        '-install_name', '@rpath/libVendor.dylib', str(source),
                        '-L' + str(frameworks), '-lFixture', '-o', str(plugin)], check=True)
        (app / 'Contents/Info.plist').write_bytes(plistlib.dumps({
            'CFBundleIdentifier': 'test.videodock.fixture', 'CFBundleName': 'Fixture',
            'CFBundleExecutable': 'Fixture', 'CFBundlePackageType': 'APPL',
        }))
        subprocess.run(['codesign', '--force', '--sign', '-', str(library)], check=True)
        subprocess.run(['codesign', '--force', '--sign', '-', '--identifier', 'vendor.fixture',
                        '--requirements', '=designated => identifier "vendor.fixture"', str(plugin)], check=True)
        subprocess.run(['codesign', '--force', '--sign', '-', str(app)], check=True)
        subprocess.run(['codesign', '--verify', '--deep', '--strict', str(app)], check=True)
        module.repair(app, '-')
        subprocess.run(['codesign', '--verify', '--strict', '-R', '=identifier "vendor.fixture"', str(plugin)], check=True)
        print('macOS signing regression: dependency repair preserves nested code identity and requirement')
print('macOS framework dependency regression: passed')
