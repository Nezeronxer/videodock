"""Installer failure contracts using temporary profiles and command stubs only."""
import os
from pathlib import Path
import subprocess
import tempfile
import unittest


PROJECT = Path(__file__).resolve().parents[3]


class InstallStagingTests(unittest.TestCase):
    def install(self, platform, failure=None, existing=True):
        temporary = tempfile.TemporaryDirectory(prefix='videodock-install-test-')
        self.addCleanup(temporary.cleanup)
        root = Path(temporary.name)
        home, tools, target = (root / name for name in ('home', 'tools', 'build'))
        for path in (home, tools, target / 'release'):
            path.mkdir(parents=True)
        destination = (home / 'Applications/VideoDock.app' if platform == 'Darwin'
                       else home / '.local/lib/videodock')
        executable = (destination / 'Contents/MacOS/VideoDock' if platform == 'Darwin'
                      else destination / 'videodock')
        if existing:
            executable.parent.mkdir(parents=True)
            executable.write_text('previous working executable')
            (destination / 'previous-resource').write_text('previous resource')
        scripts = {
            'uname': f'echo {platform}',
            'cargo': 'printf "new executable" > "$CARGO_TARGET_DIR/release/videodock-desktop"',
            'ffmpeg': '[ "$INSTALL_TEST_FAILURE" != notices ] || exit 93',
            'ffprobe': 'exit 0',
            'git': 'exit 0',
            'brew': 'exit 0',
            'qmake': 'echo 6.8.3',
            'qmake6': 'echo 6.8.3',
            'macdeployqt': '[ "$INSTALL_TEST_FAILURE" != deploy ] || exit 94\nmkdir -p "$1/Contents/Frameworks"',
            'otool': 'printf "%s:\\n\\t/usr/lib/libSystem.B.dylib (compatibility version 1.0.0)\\n" "$2"',
            'plutil': 'exit 0',
            'install_name_tool': 'exit 0',
            'codesign': '[ "$INSTALL_TEST_FAILURE" != signing ] || exit 95',
            'python3': 'exit 0',
            'mv': '''if [ "$INSTALL_TEST_FAILURE" = publish ] && [ "$2" = "$INSTALL_TEST_DESTINATION" ]; then
    case "$1" in */previous) ;; *) exit 96;; esac
fi
exec /bin/mv "$@"''',
            'install': '''case "$*" in *icons/hicolor*) [ "$INSTALL_TEST_FAILURE" != desktop ] || exit 97;; esac
if [ "$1" = -Dm755 ] || [ "$1" = -Dm644 ]; then
    mode=${1#-Dm}; shift
    mkdir -p "$(dirname "$2")"
    cp "$1" "$2"
    chmod "$mode" "$2"
else
    exec /usr/bin/install "$@"
fi''',
        }
        for name, body in scripts.items():
            path = tools / name
            path.write_text('#!/bin/sh\n' + body + '\n')
            path.chmod(0o755)
        environment = dict(os.environ, HOME=str(home), CARGO_TARGET_DIR=str(target),
                           VIDEODOCK_MAC_APP=str(destination),
                           INSTALL_TEST_FAILURE=failure or '',
                           INSTALL_TEST_DESTINATION=str(destination),
                           INSTALL_TEST_TOOLS=str(tools),
                           PATH=str(tools) + os.pathsep + os.environ.get('PATH', ''))
        # Shell functions keep real Qt binaries from overriding stubs after PATH setup.
        override_names = set(scripts) - {'uname', 'cargo', 'ffmpeg', 'ffprobe', 'git'}
        overrides = '\n'.join(f'{name}() {{ "$INSTALL_TEST_TOOLS/{name}" "$@"; }}'
                              for name in sorted(override_names))
        result = subprocess.run(['sh', '-c', overrides + '\n. "$0"',
                                 str(PROJECT / 'scripts/rust/install.sh')],
                                env=environment, capture_output=True, text=True)
        return result, executable, destination

    def test_notices_failure_preserves_previous_install(self):
        for platform in ('Darwin', 'Linux'):
            with self.subTest(platform=platform):
                result, executable, destination = self.install(platform, 'notices')
                self.assertEqual(result.returncode, 93, result.stderr)
                self.assertEqual(executable.read_text(), 'previous working executable')
                self.assertTrue((destination / 'previous-resource').is_file())

    def test_deployment_failure_preserves_previous_mac_app(self):
        result, executable, _ = self.install('Darwin', 'deploy')
        self.assertEqual(result.returncode, 94, result.stderr)
        self.assertEqual(executable.read_text(), 'previous working executable')

    def test_publish_failure_restores_previous_install(self):
        for platform in ('Darwin', 'Linux'):
            with self.subTest(platform=platform):
                result, executable, _ = self.install(platform, 'publish')
                self.assertNotEqual(result.returncode, 0, result.stderr)
                self.assertEqual(executable.read_text(), 'previous working executable')

    def test_signing_failure_preserves_previous_mac_app(self):
        result, executable, _ = self.install('Darwin', 'signing')
        self.assertEqual(result.returncode, 95, result.stderr)
        self.assertEqual(executable.read_text(), 'previous working executable')

    def test_desktop_failure_restores_previous_linux_install(self):
        result, executable, _ = self.install('Linux', 'desktop')
        self.assertEqual(result.returncode, 97, result.stderr)
        self.assertEqual(executable.read_text(), 'previous working executable')

    def test_publish_failure_without_previous_install_leaves_no_partial_install(self):
        for platform in ('Darwin', 'Linux'):
            with self.subTest(platform=platform):
                result, _, destination = self.install(platform, 'publish', existing=False)
                self.assertEqual(result.returncode, 96, result.stderr)
                self.assertFalse(destination.exists())
                self.assertEqual(list(destination.parent.glob('.videodock-install.*')), [])

    def test_success_replaces_complete_install_and_cleans_staging(self):
        for platform in ('Darwin', 'Linux'):
            with self.subTest(platform=platform):
                result, executable, destination = self.install(platform)
                self.assertEqual(result.returncode, 0, result.stderr)
                self.assertEqual(executable.read_text(), 'new executable')
                licenses = (destination / 'Contents/Resources/licenses' if platform == 'Darwin'
                            else destination / 'licenses')
                self.assertTrue((licenses / 'GPL-3.0.txt').is_file())
                self.assertFalse((destination / 'previous-resource').exists())
                self.assertEqual(list(destination.parent.glob('.videodock-install.*')), [])
                if platform == 'Linux':
                    self.assertEqual((destination.parents[1] / 'bin/videodock').resolve(), executable.resolve())


if __name__ == '__main__':
    unittest.main()
