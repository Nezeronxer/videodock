# Vendored source and changes

Source: TeamPiped/rustypipe commit 3f1491263929c22036429b665e370fdd2f066017, https://github.com/TeamPiped/rustypipe/tree/3f1491263929c22036429b665e370fdd2f066017 . Original ThetaDev and contributor attribution, GPL-3.0 LICENSE and Rust-port yt-dlp/ejs Unlicense attribution in deobfuscate/solver.rs are preserved.

Local changes: lossless player projectionType/stereoLayout response-to-stream metadata and isolated raw-response mapping regression; flattened package manifest without unrelated upstream workspace members; wreq pinned=0.16.3 with explicit Tokio/query/stream features and adapted URI/body APIs (removes lru0.13 soundness advisories), flexon pinned=0.4.6 (0.4.9 fails stable1.99 borrow checking); direct modern playlist lockup lists; continuationItemViewModel wrappers; modern duration badge aliases; sanitized regression fixture; opt-in current YouTube player sig/nsig live test. Unrelated103MB upstream network fixtures are not included; their original tests require upstream checkout. Modern included fixture test is self-contained.

Additional local APIs: `attested_player` accepts a caller-supplied anonymous visitor/proof and returns a bounded raw player response plus client version; `decipher_stream_url` evaluates the current player throttling signature inside Rust without requesting the delivery URL. VideoDock owns anonymous proof lifecycle and SABR transfer outside this dependency.

Verified: source build, live current player signature/nsig extraction+execution, iOS public video formats, HTTP206 media body, local ffprobe, WEBVTT subtitles, full104-item public playlist continuation. Six old playlist fixtures plus modern fixture passed in research checkout. The integrated anonymous PO/SABR path now has complete short and 635-second 720p60 download/decode evidence in `docs/youtube-delivery-2026-11-04.md`; this does not establish full YouTube/platform coverage.

Research evidence and reproducible source patch: /tmp/videodock-rust-youtube-probe/repair/ .
