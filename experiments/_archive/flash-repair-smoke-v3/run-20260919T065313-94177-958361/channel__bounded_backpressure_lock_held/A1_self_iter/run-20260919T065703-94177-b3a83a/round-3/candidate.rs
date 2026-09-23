Looking at this program:

- Channel capacity is 1.
- Sender: `send(1)` (succeeds, buffer now full), then locks/unlocks mutex, then `send(2)`.
- Receiver: `recv()` (gets 1, buffer empty), then locks/unlocks mutex, then `recv()` (gets 2).

Let me trace interleavings:

1. Sender sends 1 → buffer full.
2. Sender locks mutex, unlocks.
3. Sender sends 2 → blocks because buffer full (receiver hasn't recv'd yet).
4. Receiver recv's 1 → buffer empty, sender's send(2) completes.
5. Receiver locks mutex, unlocks.
6. Receiver recv's 2 → gets it.

No deadlock: the sender never holds the mutex while sending (the lock guard is dropped before `send(2)`). The receiver never holds the mutex while receiving. So neither blocks on the channel while holding the lock.

The mutex is only ever held briefly and never across a channel operation. There's no lock ordering issue since there's only one mutex.

Every interleaving terminates, and it prints `DONE done=1`.

NO_ISSUES
