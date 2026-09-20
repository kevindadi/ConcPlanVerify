# post-edit-conform-v1 — SUMMARY

- requests: 57/60
- binary sha256: `03d343e558854d65934ee0a2ea00d522d4dca102966d8c499fb4367d37b583aa`

| edit | cells | build_ok | conform PASS | miri detected | behavior hang | instrument_limit | drift_caught_only_by_conform |
| --- | --- | --- | --- | --- | --- | --- | --- |
| E1 | 19 | 19 | 19 | 0 | 0 | 0 | 0 |
| E2 | 19 | 14 | 14 | 0 | 0 | 0 | 0 |
| E3 | 19 | 17 | 17 | 0 | 0 | 0 | 0 |

## drift_caught_only_by_conform (tool-green but off-model)

| task | arm | edit | conform reason | ev orig/edited |
| --- | --- | --- | --- | --- |
