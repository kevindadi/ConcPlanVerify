# conform-mutation-v2 — SUMMARY

- programs: 23; mutants: 99
- binary sha256: `073129de3a6d378a4e198bf712028c0c950cdf080981fb0073dfb7af5fa7ce5c`
- M3 (move ev) is n/a by construction in v2 (no standalone ev).

| op | mutants | build_ok | conform FAIL | recall | reasons | miri | hang |
| --- | --- | --- | --- | --- | --- | --- | --- |
| M1 | 13 | 13 | 13 | 1.0 | {'violation': 11, 'timeout': 2} | 0 | 2 |
| M2 | 19 | 19 | 18 | 0.947 | {'violation': 16, 'timeout': 2} | 0 | 2 |
| M4 | 5 | 5 | 5 | 1.0 | {'violation': 5} | 0 | 1 |
| M5 | 6 | 6 | 0 | 0.0 | {} | 0 | 0 |
| M6 | 23 | 23 | 23 | 1.0 | {'violation': 22, 'timeout': 1} | 0 | 1 |
| M7 | 23 | 23 | 2 | 0.087 | {'violation': 2} | 0 | 0 |
| M8 | 10 | 10 | 8 | 0.8 | {'violation': 8} | 0 | 0 |
