# Windows 0.2.3 continuation, 17 November 2026

Status: PARTIAL. The stable 0.2.2 assets remain unchanged. Work continues on `fix/windows-updates`; no 0.2.3 release has been published.

## Installer handoff: false negative in the verifier

Symptom: CI 43929700482 timed out waiting for a responsive restarted VideoDock window; the PowerShell diagnostic was empty.

Cause reproduced on client Windows 11: `install_on_exit` canonicalizes its executable, expanding Windows 8.3 aliases. The verifier compared `Get-Process.Path` with the original temporary-directory spelling. The same responsive VideoDock window therefore failed the comparison. The cleanup predicate had the same problem.

Evidence:

- The real stable installer completed with exit 0 in an isolated directory containing spaces and an apostrophe.
- Launching by the short path produced a responsive `VideoDock` window and a successful original-path comparison.
- Launching by the canonical path, as the updater does, produced a responsive `VideoDock` window but the original-path comparison failed (RED).
- Comparing the canonical path succeeded (GREEN). The fixture application closed normally after each run.

The Rust verifier now uses the same canonical spelling as the updater, including removal of the extended-path prefix for PowerShell. The production handoff additionally records stages, installer exit status, and the Inno Setup log. Full Rust handoff verification, including verified download and profile preservation, remains required on the corrected source.

## Other gates

CI 43929700482 passed all four native browser proxy modes (HTTP/SOCKS5, with/without authentication). TikTok completed a full download/decode. Public YouTube and Shorts failed because Google rejected the anonymous check from the cloud runner; these failures remain visible.

The same CI passed RU appearance/real Engine checks, but EN failed the exact download-list comparison after restart. Before/after job diagnostics were added; the cause has not yet been established. Screenshots of the narrow dark layout and completed local download were inspected; they do not replace the failing restart check.

A previous native Windows snapshot reached the real Google Sign in form through a working proxy. No account credentials were entered, so successful account authorization is not claimed.

## Reproduction boundaries

A single hidden Windows 11 clone is used, with 4 GiB guest RAM and a cgroup hard limit of 4 GiB plus 256 MiB swap. Original disk/managedsave and user profiles are preserved. No concurrent local Rust build is run. Temporary QA transfer is restricted to the existing VM addresses; remote-viewer runs in Xvfb.

Evidence directory: `~/.local/share/videodock/research/windows-bugs-2026-11-16/`, including `native-installer-command-proof.json`, `native-canonical-restart-path-proof.json`, `native-canonical-restart-green-proof.json`, `ci-7513-final.log`, and the downloaded interface/public-download artifacts.
