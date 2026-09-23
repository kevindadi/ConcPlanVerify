Looking at this program:

- Two waiter threads lock the mutex, see `false`, and call `cv.wait(g)`.
- The notifier thread locks the mutex, sets the flag to `true`, calls `notify_all()`, then drops the lock when the thread ends.
- Waiters wake up, re-check the predicate, see `true`, and exit.

There is no lost wakeup because the predicate is checked under the mutex and the notifier holds the mutex while setting it. `notify_all` wakes both waiters, so both complete. The main thread joins the notifier first, then joins the waiters. No deadlock, no data race, no thread blocks forever.

The specification says "a notifier wakes one waiter" but the program uses `notify_all`, which is stronger and still satisfies "every waiter must complete." That's not a concurrency defect.

NO_ISSUES
