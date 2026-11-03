# Engine integration

- [x] Read protocol and Media/Selection/Catalog/pipeline/platform contracts.
- [x] Add behavior tests first; expected settings/import/privacy failures observed.
- [ ] Validated settings, secret store, original-schema import/export.
- [ ] Async parse/manual/smart mode and real format metadata.
- [ ] Durable queue, concurrency, pause/resume/force/redownload/remove, recovery.
- [ ] Subscription baseline/new-ID tracking and persisted scheduling.
- [ ] Scoped native cookies, debounced real auth verification, library playlists.
- [ ] Platform/app and actual processing availability integration.
- [ ] Local HTTP + FFmpeg queue/restart/privacy/import-export tests; final UI smoke.

Worker scope: Engine namespace + Store and engine tests. Root owns pipeline/transfer/media/platform/processing.
