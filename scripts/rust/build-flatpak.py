#!/usr/bin/env python3
"""Build against KDE SDK and matching Qt WebEngine, offline and with a real memory limit."""
import argparse
import os
from pathlib import Path
import subprocess
import time
import uuid

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('baseapp_build', type=Path)
parser.add_argument('vendor_registry', type=Path)
parser.add_argument('rust_prefix', type=Path)
args = parser.parse_args()
project = Path(__file__).resolve().parents[2]
base, vendor, rust = (p.resolve(strict=True) for p in (args.baseapp_build, args.vendor_registry, args.rust_prefix))
assert (base / 'files/lib/libexec/QtWebEngineProcess').is_file()
assert (rust / 'bin/rustc').is_file() and (rust / 'bin/cargo').is_file()
cache = Path.home() / '.cache/videodock-flatpak'
target, cargo = cache / 'sdk-target', cache / 'sdk-cargo'
target.mkdir(parents=True, exist_ok=True)
cargo.mkdir(parents=True, exist_ok=True)
(cargo / 'config.toml').write_text('[source.crates-io]\nreplace-with="offline-vendor"\n[source.offline-vendor]\ndirectory="/run/videodock-vendor"\n')
marker = uuid.uuid4().hex
filename = '.allow-sdk-build-' + marker
runtime = os.environ.get('XDG_RUNTIME_DIR', f'/run/user/{os.getuid()}')
env = {**os.environ, 'XDG_RUNTIME_DIR': runtime,
       'DBUS_SESSION_BUS_ADDRESS': os.environ.get('DBUS_SESSION_BUS_ADDRESS', f'unix:path={runtime}/bus')}
command = ['flatpak', 'build', '--die-with-parent', '--unshare=network']
for destination, source in [('/run/videodock-source', project), ('/run/videodock-target', target),
                            ('/run/videodock-cargo', cargo), ('/run/videodock-toolchain', rust),
                            ('/run/videodock-vendor', vendor), ('/run/videodock-tools', project / 'packaging/linux')]:
    command.append(f'--bind-mount={destination}={source}')
command.append('--build-dir=/run/videodock-source')
variables = {
    'PATH': '/run/videodock-toolchain/bin:/app/bin:/usr/bin',
    'CARGO_HOME': '/run/videodock-cargo', 'CARGO_TARGET_DIR': '/run/videodock-target',
    'CARGO_BUILD_JOBS': '1', 'CMAKE_BUILD_PARALLEL_LEVEL': '1',
    'QMAKE': '/run/videodock-tools/qmake-sdk.sh', 'QMAKEPATH': '/app',
    'LIBCLANG_PATH': '/usr/lib/sdk/llvm22/lib',
    'BINDGEN_EXTRA_CLANG_ARGS': '-isystem /usr/lib/sdk/llvm22/lib/clang/22/include',
    'PKG_CONFIG_PATH': '/app/lib/x86_64-linux-gnu/pkgconfig:/app/lib/pkgconfig:/usr/lib/x86_64-linux-gnu/pkgconfig',
    'LD_LIBRARY_PATH': '/app/lib/x86_64-linux-gnu:/app/lib',
    'CXXFLAGS': '-I/app/include -I/app/include/QtWebEngineQuick -I/app/include/QtWebEngineCore -I/app/include/QtWebEngineWidgets',
    'RUSTFLAGS': '-L native=/app/lib/x86_64-linux-gnu -L native=/app/lib -C link-arg=-Wl,-rpath,/app/lib/x86_64-linux-gnu -C link-arg=-Wl,-rpath,/app/lib',
    'VD_SDK_BUILD_MARKER': marker, 'VD_SDK_GATE': '/run/videodock-target/' + filename,
}
command.extend(f'--env={key}={value}' for key, value in variables.items())
script = 'i=0; while [ "$i" -lt 30 ]; do if [ -f "$VD_SDK_GATE" ]; then break; fi; i=$((i+1)); sleep 1; done; test -f "$VD_SDK_GATE" || exit 1; sh /run/videodock-tools/prepare-qt-sdk.sh; exec cargo build --release --locked --offline -p videodock-desktop'
command.extend([str(base), 'sh', '-ec', script, 'videodock-sdk-build', marker])
child = subprocess.Popen(command, env=env)
try:
    unit = None
    for _ in range(100):
        for proc in Path('/proc').iterdir():
            if not proc.name.isdigit():
                continue
            try:
                if marker.encode() not in (proc / 'cmdline').read_bytes().split(b'\0'):
                    continue
                if (proc / 'comm').read_text().strip() != 'sh':
                    continue
                group = (proc / 'cgroup').read_text().strip().split('::', 1)[1]
                candidate = Path(group).name
                if candidate.startswith('app-flatpak-io.github.Nezeronxer.VideoDock-') and candidate.endswith('.scope'):
                    unit = candidate
                    break
            except (OSError, IndexError):
                continue
        if unit:
            break
        if child.poll() is not None:
            raise RuntimeError('SDK startup failed')
        time.sleep(.1)
    assert unit, 'Own SDK cgroup was not identified; build remains stopped'
    subprocess.run(['systemctl', '--user', 'set-property', '--runtime', unit,
                    'MemoryHigh=960M', 'MemoryMax=1G', 'MemorySwapMax=0', 'CPUWeight=20'], env=env, check=True)
    cgroup = Path('/sys/fs/cgroup' + group)
    assert (cgroup / 'memory.max').read_text().strip() == '1073741824'
    assert (cgroup / 'memory.swap.max').read_text().strip() == '0'
    print('SDK build: 1 GiB memory limit, no swap, one build job', flush=True)
    (target / filename).touch(exist_ok=False)
    raise SystemExit(child.wait())
finally:
    if child.poll() is None:
        child.terminate()
