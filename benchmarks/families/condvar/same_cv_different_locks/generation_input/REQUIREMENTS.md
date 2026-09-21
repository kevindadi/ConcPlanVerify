# Requirements

R1. A main task starts two waiter roles and one notifier role that run at the same time. [U]
R2. The two waiters guard their work with two different locks.
R3. Both waiters block on the same condition variable.
R4. A waiter must hold its own lock while it waits on the shared condition variable.
R5. Each waiter must announce that it is about to wait before it blocks, so the notifier knows how many waits to expect.
R6. The notifier must wait until both waiters have announced themselves before it wakes them.
R7. When the notifier wakes the waiters it must hold each lock that a waiter needs in order to wake and finish.
R8. The waking step must not leave any waiter blocked and must not depend on a race between the waiters.
R9. Every schedule and interleaving must terminate with all roles finished.
R10. The program must print exactly the line `DONE done=1` and then exit. [U]
