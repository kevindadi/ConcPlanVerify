# Extraction v4 — SUMMARY

- binary sha256: `88c3217d4b8f6549c8f7e07787e384ea3b065dc99abab47a27766beb958b42a3`
- protocol sha256: `f7ce2844fa588ef85064fdd5cbd7b72787d2dd9b4d2f01e4944d75ddf7224f94`
- requests: 8/64
- validated: 1
- harness errors: 0
- stage distribution: {'explore': 1, 'conform': 4, 'codegen': 3}

| task | stage | validated | model verdict |
| --- | --- | --- | --- |
| lock-order/partial_deadlock_bystander | explore | True | INVALID |
| lock-order/cross_module_cycle | conform | False | None |
| lock-order/cycle_3lock | conform | False | None |
| structure/nested_scope_lock_order | codegen | False | None |
| condvar/notify_one_multi_waiter_wrong_pick | codegen | False | None |
| channel/bounded_backpressure_lock_held | conform | False | None |
| channel/send_while_holding_mutex | conform | False | None |
| semaphore/acquire_twice_no_release | codegen | False | None |
