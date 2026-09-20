# conform-mutation-v1 — SUMMARY

- programs: 19; mutants: 86
- binary sha256: `03d343e558854d65934ee0a2ea00d522d4dca102966d8c499fb4367d37b583aa`
- conform FAIL reasons: `violation`/`unknown_sid` = event-stream mismatch; `timeout` = the mutant hangs (behavior catches it, not conform's event check).

| op | mutants | build_ok | conform FAIL | recall | reasons | miri detected | behavior hang |
| --- | --- | --- | --- | --- | --- | --- | --- |
| M1 | 11 | 11 | 6 | 0.545 | {'timeout': 6} | 0 | 2 |
| M2 | 15 | 15 | 1 | 0.067 | {'timeout': 1} | 0 | 1 |
| M3 | 19 | 19 | 0 | 0.0 | {} | 0 | 0 |
| M4 | 3 | 3 | 1 | 0.333 | {'timeout': 1} | 0 | 1 |
| M6 | 19 | 19 | 19 | 1.0 | {'violation': 19} | 0 | 0 |
| M7 | 19 | 19 | 2 | 0.105 | {'violation': 2} | 0 | 0 |
