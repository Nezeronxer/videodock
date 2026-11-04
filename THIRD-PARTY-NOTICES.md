# VideoDock distribution notices

VideoDock is distributed under GNU GPL version 3; see `licenses/GPL-3.0.txt`.
The corresponding application source, dependency versions, build scripts and
local RustyPipe changes are published with the matching VideoDock release at
https://github.com/Nezeronxer/videodock/releases . The `Cargo.lock` file pins Rust
dependencies; `licenses` includes their notices and license texts.

RustyPipe copyright ThetaDev and contributors, GNU GPL-3.0:
https://github.com/TeamPiped/rustypipe . Its license is included, and the matching
source contains the modified dependency and `vendor/rustypipe/PATCHES.md`.

BgUtils copyright LuanRT, MIT; source commit
f842f8a14435a35800ff469d315bb1a7e6d7a64a:
https://github.com/LuanRT/BgUtils . `licenses/BgUtils-MIT.txt` contains its notice.
The unchanged source subset and reproducible adapter are included in VideoDock's
source, and its notice is also embedded in the executable.

SABR protocol references: LuanRT/googlevideo, copyright 2024 LuanRT, and
mthwJsmith/sabr-rs, copyright 2026 mthwJsmith, both MIT:
https://github.com/LuanRT/googlevideo and https://github.com/mthwJsmith/sabr-rs .
Their complete notices are in `licenses/googlevideo-MIT.txt` and
`licenses/sabr-rs-MIT.txt`.

Qt is dynamically linked. Qt copyright The Qt Company Ltd. and contributors;
the included modules are used under their open source licenses. GNU LGPL 2.1,
LGPL 3 and GPL 3 texts are supplied in `licenses`. The package's Qt version and
FFmpeg configuration are recorded beside these notices. Corresponding Qt source
releases: https://download.qt.io/archive/qt/ . Qt WebEngine incorporates Chromium
and other third-party components; their bundled attribution page is available
through the built-in browser at `chrome://credits`.

FFmpeg copyright the FFmpeg developers: https://ffmpeg.org/ . Its actual license
and enabled components are recorded in `licenses/ffmpeg-license.txt` and
`licenses/ffmpeg-buildconf.txt`; source releases are available at
https://ffmpeg.org/releases/ . Platform package provenance is recorded separately
for each released build. The bundled FFmpeg executable is a separate program,
invoked only for local media processing. On Linux Qt and FFmpeg are provided by
the operating system, and their package licenses remain installed there.

Users may replace dynamically linked libraries and the separate FFmpeg tools
with compatible modified versions. VideoDock does not impose an additional
restriction on that replacement. This software is provided without warranty;
the full license terms and original copyright notices govern each component.
