## Repair Strategy: Channel Block — Move Blocking Operations Outside Locks

A channel block occurs when a `send` or `recv` operation on a channel cannot complete because the matching counterpart is blocked. This commonly happens when both sender and receiver hold the same mutex while trying to use the channel, creating a deadlock between mutex and channel.

### Fix Rules

1. Never perform a blocking channel operation (`send` or `recv`) while holding a mutex.
2. If you need mutex-protected data for the send, read it under the lock first, drop the lock, then send.
3. If you need to recv and then update mutex-protected data, recv first without the lock, then lock and update.
4. Ensure send/recv are properly paired: every send must have a matching recv.

### Example: Buggy ConcIR (channel block)

Both sender and receiver hold `mtx` while performing channel operations. If receiver locks first, it blocks on `recv` while holding `mtx`, preventing sender from locking `mtx` to send:

```json
{
  "name": "sender", "kind": "closure",
  "body": [
    {"sid": "s1", "op": ["res_op", "mtx", "lock"],      "transfer": ["next", "s2"]},
    {"sid": "s2", "op": ["res_op", "ch", "send", "42"],  "transfer": ["next", "s3"]},
    {"sid": "s3", "op": ["res_op", "mtx", "drop"],       "transfer": ["next", "s4"]},
    {"sid": "s4", "op": "return",                         "transfer": "return"}
  ]
}
```

```json
{
  "name": "receiver", "kind": "closure",
  "body": [
    {"sid": "s1", "op": ["res_op", "mtx", "lock"],      "transfer": ["next", "s2"]},
    {"sid": "s2", "op": ["res_op", "ch", "recv"],        "transfer": ["next", "s3"]},
    {"sid": "s3", "op": ["res_op", "mtx", "drop"],       "transfer": ["next", "s4"]},
    {"sid": "s4", "op": "return",                         "transfer": "return"}
  ]
}
```

### Fixed ConcIR

Move `recv` before the lock acquisition in the receiver:

```json
{
  "name": "receiver", "kind": "closure",
  "body": [
    {"sid": "s1", "op": ["res_op", "ch", "recv"],        "transfer": ["next", "s2"]},
    {"sid": "s2", "op": ["res_op", "mtx", "lock"],       "transfer": ["next", "s3"]},
    {"sid": "s3", "op": ["res_op", "mtx", "drop"],       "transfer": ["next", "s4"]},
    {"sid": "s4", "op": "return",                         "transfer": "return"}
  ]
}
```

The fix moves `recv` outside the locked section so the receiver does not hold `mtx` while waiting for data. The sender can now acquire `mtx` and complete the `send`.
