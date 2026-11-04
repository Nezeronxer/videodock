The embedded browser script uses BgUtils 4.0.3, upstream commit
`f842f8a14435a35800ff469d315bb1a7e6d7a64a` (MIT, LuanRT).
The unchanged source subset and full license are in `bgutils/`.

`entry.ts` is VideoDock's bounded, anonymous proof adapter. Rust owns metadata,
signatures, source selection and every media transfer. The browser worker has
an off-record profile and no access to the normal application browser profile.

The private protocol allows direct-only DNS bootstrap entries for exactly
`www.youtube.com` and `www.google.com`, validated public IPv4 addresses from
Rust. Chromium receives individual hostname mapping rules. The separate Qt
interpreter request keeps its original redirect validation, TLS peer name and
HTTP Host while dialing the supplied address; HTTP/2 is disabled because its
authority otherwise follows the dial address. A selected proxy disables all
bootstrap mappings. HTTP proxy credentials remain in the private request and
are supplied only to the selected proxy, with bounded authentication retries;
website authentication receives none. HTTPS and authenticated SOCKS proof
proxies fail closed.

The checked-in bundle needs no JavaScript tool at application runtime or normal
Cargo build. To reproduce it with esbuild 0.28.2:

```
esbuild crates/desktop/attestation/entry.ts --bundle --format=iife --target=chrome120 --charset=utf8 --legal-comments=inline --outfile=crates/desktop/attestation/attest.js
```

The MIT license is embedded in the executable alongside this bundle. The public
Google API client identifier in the upstream constants is not an account secret.

Offline checks: `cargo test -p videodock-desktop`,
`sh crates/desktop/tests/check-proof-boundaries.sh`, and
`python3 crates/desktop/tests/proof_worker.py <desktop-executable> --proxy --startup`.
The opt-in live check `proof_http_auth.py <desktop-executable> <existing-socks-host:port>`
uses a temporary local authenticated proxy to verify anonymous proof and bounded
wrong-password failure. It closes every listener and tunnel after the check.
