"""Authored generation-requirement documents for benchmark v3 (§0).

Each entry is a numbered requirement document for a *generation* task
(requirements -> program) for the six capability families.  The document is
split into ``R1..Rn`` items covering three kinds of statement: functional
(who does what / how data flows), concurrency (mutual exclusion, ordering,
handshake, backpressure, capacity, notification intent), and termination with
an observable terminal status line.

``unverifiable`` lists the 1-based indices of requirements that no contract
clause can decide (e.g. purely functional statements, or the terminal line,
which the trace monitor cannot observe); they are rendered with an ``[U]``
marker and excluded from requirement coverage.

``clauses`` maps each frozen-contract clause to the requirement ids it makes
checkable, indexed in the same order as ``contract.json``'s ``properties`` and
``preserved`` arrays.

Provenance: drafted by author-proxy agents from the frozen contracts and
design briefs, reviewed and corrected by the benchmark owner.  The text must
stay free of ConcIR/CIR/sid/contract terminology and of code.
"""

from __future__ import annotations

GENERATION: dict[str, dict] = {
    "lock-order/abba_2lock": {
        "requirements": [
            "The design contains one main thread and two worker threads.",
            "Both worker threads share the same two locks, and neither lock may be held by more than one worker at a time.",
            "Each worker must hold both locks at the same time while it performs its critical work.",
            "A worker that finds a lock busy must wait for it to become free and then continue.",
            "It must never be possible for the two workers to leave each other waiting forever, each holding one lock and needing the other.",
            "The main thread starts both workers and only finishes after both workers have finished.",
            "Every worker must release each lock it holds before it finishes.",
            "Every possible schedule or interleaving of the workers must terminate.",
            "On completion the program must print exactly one final status line reporting that both workers finished, and then exit.",
        ],
        "unverifiable": [9],
        "clauses": {
            "properties": [["R4", "R5", "R7", "R8"]],
            "preserved": [
                ["R1", "R6", "R8"],
                ["R1", "R6", "R8"],
                ["R2", "R3"],
                ["R2", "R3"],
            ],
        },
    },
    "lock-order/cycle_3lock": {
        "requirements": [
            "The design contains one main thread and three worker threads.",
            "Three locks are shared among the workers, and none of them may be held by more than one worker at a time.",
            "The first worker needs the first and second lock, the second worker needs the second and third lock, and the third worker needs the third and the first lock.",
            "Each worker must hold its two locks at the same time while it performs its critical work.",
            "It must never be possible for the workers to leave one another waiting forever, each holding one lock and needing another.",
            "Every worker must release each lock it holds before it finishes.",
            "The main thread starts all three workers and only finishes after all three have finished.",
            "Every worker must finish eventually, whatever order the workers run in.",
            "Every possible schedule or interleaving of the workers must terminate.",
            "On completion the program must print exactly one final status line reporting that all three workers finished, and then exit.",
        ],
        "unverifiable": [10],
        "clauses": {
            "properties": [["R5", "R6", "R8", "R9"]],
            "preserved": [
                ["R1", "R7", "R8"],
                ["R1", "R7", "R8"],
                ["R1", "R7", "R8"],
                ["R2", "R3", "R4"],
                ["R2", "R3", "R4"],
                ["R2", "R3", "R4"],
            ],
        },
    },
    "lock-order/cross_module_cycle": {
        "requirements": [
            "The design contains two modules, where one module owns the first shared resource and the other module owns the second shared resource.",
            "One task runs in the first module and another task runs in the second module, and both tasks need both resources.",
            "Each task must declare that it depends on the resource owned by the other module.",
            "Each task must hold both resources at the same time while it performs its work.",
            "It must never be possible for the two tasks to leave each other waiting forever, each holding one resource and needing the other.",
            "Every task must release each resource it holds before it finishes.",
            "A starting thread launches both tasks and only finishes after both tasks have finished.",
            "Both tasks must finish even though they run in different modules.",
            "Every possible schedule or interleaving must terminate.",
            "On completion the program must print exactly one final status line reporting that both tasks finished, and then exit.",
        ],
        "unverifiable": [10],
        "clauses": {
            "properties": [["R5", "R6", "R8", "R9"]],
            "preserved": [
                ["R2", "R7", "R8"],
                ["R2", "R7", "R8"],
                ["R1", "R3", "R4"],
                ["R1", "R3", "R4"],
            ],
        },
    },
    "lock-order/partial_deadlock_bystander": {
        "requirements": [
            "The design contains one main thread and two short-lived workers.",
            "An independent bystander task also runs and keeps making progress without ever finishing on its own.",
            "The two workers share the same two locks and also use two counting permits to coordinate with each other.",
            "Each worker must hold both locks at the same time while it enters its critical section.",
            "The two workers must use the two permits as a handshake, so that neither takes its second lock before the other has taken its first.",
            "The bystander task must not prevent the two workers from finishing.",
            "At every point during execution, it must still be possible for the first worker to finish.",
            "At every point during execution, it must still be possible for the second worker to finish.",
            "Neither worker may wait forever for a permit or a lock that the other worker never provides.",
            "Every worker must release each lock it holds before it finishes.",
            "The main thread starts all three tasks and only finishes after both workers have finished, even though the bystander may keep running.",
            "On completion the program must print exactly one final status line reporting that both workers finished, and then exit.",
        ],
        "unverifiable": [2, 12],
        "clauses": {
            "properties": [
                ["R6", "R9", "R10", "R11"],
                ["R6", "R7", "R11"],
                ["R6", "R8", "R11"],
            ],
            "preserved": [
                ["R1", "R7", "R11"],
                ["R1", "R8", "R11"],
                ["R3", "R4", "R5"],
                ["R3", "R4", "R5"],
            ],
        },
    },
    "lock-order/two_independent_cycles": {
        "requirements": [
            "The design contains one main thread and four worker threads.",
            "Four locks are shared among the workers, arranged as a first pair and a second pair.",
            "The first two workers each need both locks of the first pair, and the last two workers each need both locks of the second pair.",
            "Each worker must hold both locks of its pair at the same time while it works.",
            "The two pairs must be independent, so that progress in one pair does not depend on the other pair.",
            "Within the first pair, the two workers must take their two locks in the same relative sequence.",
            "Within the second pair, the two workers must take their two locks in the same relative sequence.",
            "No worker may hold one lock of its pair while waiting forever for the other lock of that pair.",
            "Every worker must release each lock it holds before it finishes.",
            "The main thread starts all four workers and only finishes after all four have finished.",
            "Every possible schedule or interleaving of the workers must terminate.",
            "On completion the program must print exactly one final status line reporting that all four workers finished, and then exit.",
        ],
        "unverifiable": [12],
        "clauses": {
            "properties": [["R5", "R6", "R7", "R8", "R9", "R11"]],
            "preserved": [
                ["R1", "R5", "R10", "R11"],
                ["R1", "R5", "R10", "R11"],
                ["R1", "R5", "R10", "R11"],
                ["R1", "R5", "R10", "R11"],
                ["R2", "R3", "R4", "R6"],
                ["R2", "R3", "R4", "R6"],
                ["R2", "R3", "R4", "R7"],
                ["R2", "R3", "R4", "R7"],
            ],
        },
    },
    "condvar/bare_wait_no_predicate": {
        "requirements": [
            "A main task starts a waiter role and a notifier role that run at the same time.",
            "The waiter and the notifier share one lock and one condition variable, and the lock guards a boolean flag.",
            "The notifier sets the shared flag to true while holding the lock, signals the condition variable, and then releases the lock.",
            "While holding the lock, the waiter checks the flag first, waits on the condition variable only while the flag is false, and re-checks the flag after each wake.",
            "The waiter must complete even when the notifier signals before the waiter starts waiting.",
            "The waiter must not wait on the condition variable while the flag is already true, and must not keep the lock while blocked.",
            "Every schedule and interleaving of the two roles must terminate with both roles finished.",
            "The shared flag becomes true in every schedule.",
            "The waiter must not pass its wait until the flag is true.",
            "The program finishes with exactly one final status line stating that the flag is true.",
        ],
        "unverifiable": [2, 10],
        "clauses": {
            "properties": [["R6", "R7"], ["R3", "R4", "R8", "R9"]],
            "preserved": [
                ["R1", "R4", "R5", "R6", "R7", "R9"],
                ["R1", "R3", "R7"],
                ["R3", "R8"],
            ],
        },
    },
    "condvar/lost_wakeup_notify_before_wait": {
        "requirements": [
            "A main task starts a waiter role and a notifier role that run at the same time.",
            "The waiter and the notifier share one lock, one condition variable, and one boolean flag that the lock guards.",
            "The notifier sets the shared flag to true while holding the lock, signals the condition variable, and then releases the lock.",
            "The waiter checks the flag while holding the lock, waits on the condition variable only while the flag is false, and re-checks the flag after every wake.",
            "The waiter must complete even when the notifier's signal happens before the waiter starts waiting.",
            "The flag must be set to true before the signal is issued, so a waiting role never misses the notification.",
            "The flag is read and written only while the lock is held, and the waiter releases the lock while blocked.",
            "Every schedule and interleaving must terminate with the waiter and the notifier both finished.",
            "The shared flag becomes true in every schedule.",
            "The program finishes with exactly one final status line reporting the flag value.",
        ],
        "unverifiable": [2, 10],
        "clauses": {
            "properties": [["R7", "R8"], ["R3", "R4", "R6"]],
            "preserved": [
                ["R1", "R4", "R5", "R8"],
                ["R1", "R3", "R8"],
                ["R3", "R6", "R9"],
            ],
        },
    },
    "condvar/notify_one_multi_waiter_wrong_pick": {
        "requirements": [
            "A main task starts two waiter roles and one notifier role that run at the same time.",
            "Each waiter blocks on a shared condition variable until it is told to proceed.",
            "The notifier must wake every waiter that is still blocked, so no waiter is left waiting forever.",
            "A waiter must hold the lock while it waits on the condition variable.",
            "The notifier must take the lock before it wakes the waiters and release the lock afterwards.",
            "The waiters and the notifier use a separate permit counter so that the notifier only wakes the waiters after both are ready to wait.",
            "Waking a single waiter is not enough; every blocked waiter must be woken.",
            "Every schedule and interleaving must terminate with every waiter and the notifier finished.",
            "Every waiter completes in every schedule.",
            "The program finishes with exactly one final status line stating that no waiter remains.",
        ],
        "unverifiable": [1, 10],
        "clauses": {
            "properties": [["R2", "R3", "R5", "R7", "R8", "R9"]],
            "preserved": [["R4", "R5", "R6"]],
        },
    },
    "condvar/same_cv_different_locks": {
        "requirements": [
            "A main task starts two waiter roles and one notifier role that run at the same time.",
            "Each waiter guards its own data with its own lock; the two waiters need not share one lock.",
            "Each waiter must be notified before it proceeds, and the notifier must be able to wake every waiter without depending on a race between them.",
            "A waiter must hold its own lock while it waits for the notification.",
            "Each waiter must announce that it is about to wait before it blocks, so the notifier knows how many waits to expect.",
            "The notifier must wait until both waiters have announced themselves before it wakes them.",
            "When the notifier wakes the waiters it must hold each lock that a waiter needs in order to wake and finish.",
            "The waking step must not leave any waiter blocked and must not depend on a race between the waiters.",
            "Every schedule and interleaving must terminate with all roles finished.",
            "The program finishes with exactly one final status line.",
        ],
        "unverifiable": [1, 10],
        "clauses": {
            "properties": [["R2", "R3", "R4", "R5", "R6", "R7", "R8", "R9"]],
            "preserved": [],
        },
    },
    "channel/bounded_backpressure_lock_held": {
        "requirements": [
            "A main task starts a sender role and a receiver role that run at the same time.",
            "The two roles communicate through a channel that can hold at most one value, and both roles occasionally need one shared lock.",
            "The sender passes two values in order and the receiver takes two values, so both roles can finish.",
            "When the channel is full the sender waits before it sends again, and when the channel is empty the receiver waits before it takes.",
            "No role may wait on the channel while holding the shared lock that the other role needs.",
            "Every schedule and interleaving must terminate with the sender and the receiver both finished.",
            "The sender must not pass its second value before the receiver has taken the first, because the channel holds only one value.",
            "The program finishes with exactly one final status line.",
        ],
        "unverifiable": [2, 8],
        "clauses": {
            "properties": [["R3", "R4", "R5", "R6", "R7"]],
            "preserved": [
                ["R1", "R3", "R5", "R6"],
                ["R1", "R3", "R5", "R6"],
            ],
        },
    },
    "channel/rendezvous_both_send": {
        "requirements": [
            "A main task starts one sending task and one receiving task that run at the same time.",
            "The two tasks communicate over a channel with no buffering, so a send and a take must meet to exchange the value.",
            "The sender passes one value and the receiver takes one value, so both tasks finish.",
            "No task may wait forever for a partner that never arrives.",
            "Every schedule and interleaving must terminate with both tasks finished and no value left in the channel.",
            "The channel is empty once both tasks have finished.",
            "The program finishes with exactly one final status line.",
        ],
        "unverifiable": [7],
        "clauses": {
            "properties": [["R2", "R3", "R4", "R5"]],
            "preserved": [
                ["R1", "R3", "R5"],
                ["R1", "R2", "R3", "R5", "R6"],
            ],
        },
    },
    "channel/send_while_holding_mutex": {
        "requirements": [
            "A main task starts a sender role and a receiver role that run at the same time, and both roles occasionally use one shared lock.",
            "The two roles exchange a value over a channel that requires both roles to meet.",
            "No role may wait on the channel while holding the shared lock that the other role needs.",
            "Every schedule and interleaving must terminate with the sender and the receiver both finished.",
            "The program finishes with exactly one final status line reporting success.",
        ],
        "unverifiable": [5],
        "clauses": {
            "properties": [["R1", "R2", "R3", "R4"]],
            "preserved": [],
        },
    },
    "semaphore/acquire_twice_no_release": {
        "requirements": [
            "The program starts a supervising task that launches two worker threads and waits for both of them to finish.",
            "Both worker threads share one counting permit pool that begins with exactly one permit.",
            "A worker must hold a permit while it performs its work, so the two workers never work at the same time.",
            "Each worker may acquire the permit more than once, and on every path it must release the permit exactly as many times as it acquired it before it finishes.",
            "While a worker waits for a permit, the worker currently holding it must remain able to release it so the waiting worker can eventually proceed.",
            "Every possible schedule and interleaving of the two workers must terminate.",
            "On completion the program writes exactly one final status line to standard output and then exits.",
        ],
        "unverifiable": [7],
        "clauses": {
            "properties": [["R4", "R5", "R6"]],
            "preserved": [["R1", "R6"], ["R1", "R6"], ["R2", "R3"]],
        },
    },
    "semaphore/permit_leak": {
        "requirements": [
            "The program starts a supervising task that launches two worker threads and waits for both of them to finish.",
            "Both worker threads share one counting permit pool that begins with exactly one permit.",
            "A worker must hold a permit while it performs its work, so the two workers never work at the same time.",
            "Each worker acquires one permit, performs its work, and must release that permit before it finishes.",
            "While a worker waits for the permit, the worker holding it must remain able to release it so the waiting worker can eventually proceed.",
            "Every possible schedule and interleaving of the two workers must terminate.",
            "On completion the program writes exactly one final status line to standard output and then exits.",
        ],
        "unverifiable": [7],
        "clauses": {
            "properties": [["R4", "R5", "R6"]],
            "preserved": [["R1", "R6"], ["R1", "R6"], ["R2", "R3"]],
        },
    },
    "semaphore/throttle_n_permits": {
        "requirements": [
            "The program starts a supervising task that launches three worker threads and waits for all of them to finish.",
            "All three worker threads share one counting permit pool that begins with exactly two permits.",
            "Each worker acquires one permit, performs its work, releases the permit, and then finishes.",
            "At most two workers may hold permits at the same time, so a third worker waits until a permit is released.",
            "While a worker waits for a permit, a holder must remain able to release it so the waiting worker can eventually proceed.",
            "Every possible schedule and interleaving of the three workers must terminate.",
            "On completion the program writes exactly one final status line to standard output and then exits.",
        ],
        "unverifiable": [7],
        "clauses": {
            "properties": [["R2", "R3", "R4", "R5", "R6"]],
            "preserved": [["R1", "R6"], ["R1", "R6"], ["R1", "R6"]],
        },
    },
    "atomic-data/atomic_lost_update": {
        "requirements": [
            "The program starts a supervising task that launches two worker threads and waits for both of them to finish.",
            "Both worker threads share a single atomic counter that starts at zero.",
            "Each worker adds exactly one to the shared counter, so once both have finished the counter equals two.",
            "Each worker updates the counter with an atomic read-modify-write retry loop, so a competing update can never be silently lost.",
            "A failed update attempt must be retried rather than abandoned, so every worker's increment eventually takes effect.",
            "From every state that can occur, it must remain possible for the counter to still reach two.",
            "Each increment appears to happen in a single indivisible step, so no partial update is ever observable.",
            "Every possible schedule and interleaving of the two workers must terminate.",
            "On completion the program writes exactly one final status line to standard output and then exits.",
        ],
        "unverifiable": [9],
        "clauses": {
            "properties": [["R4", "R5", "R8"], ["R2", "R3", "R6"]],
            "preserved": [["R1", "R8"], ["R1", "R8"], ["R3", "R6", "R7"]],
        },
    },
    "atomic-data/bounded_counter_invariant": {
        "requirements": [
            "The program starts a supervising task that launches two worker threads and waits for both of them to finish.",
            "Both worker threads share a single integer counter that is declared to range from zero to two and starts at zero.",
            "Each worker adds exactly one to the shared counter while it holds the mutual-exclusion lock.",
            "A worker must hold the mutual-exclusion lock whenever it reads or writes the shared counter, so the two workers never update it at the same time.",
            "In every state that can occur, the counter value stays within its declared range from zero to two.",
            "Every possible schedule and interleaving of the two workers must terminate.",
            "On completion the program writes exactly one final status line to standard output and then exits.",
        ],
        "unverifiable": [7],
        "clauses": {
            "properties": [["R3", "R4", "R6"], ["R2", "R5"]],
            "preserved": [["R1", "R6"], ["R1", "R6"]],
        },
    },
    "atomic-data/counter_overflow_safety": {
        "requirements": [
            "The program starts a supervising task that launches two worker threads and waits for both of them to finish.",
            "Both worker threads share a single integer counter that is declared to range from zero to two and starts at zero.",
            "Each worker adds one to the shared counter only when doing so keeps the value within the required upper limit.",
            "A worker must hold the mutual-exclusion lock whenever it reads or writes the shared counter, so the two workers never update it at the same time.",
            "In every state that can occur, the counter never exceeds one, even though its declared range allows up to two.",
            "Every possible schedule and interleaving of the two workers must terminate.",
            "On completion the program writes exactly one final status line to standard output and then exits.",
        ],
        "unverifiable": [7],
        "clauses": {
            "properties": [["R3", "R4", "R6"], ["R2", "R5"]],
            "preserved": [["R1", "R6"], ["R1", "R6"]],
        },
    },
    "structure/finite_call_loop": {
        "requirements": [
            "A main task calls an auxiliary routine and then begins the same call sequence again.",
            "The main task and the auxiliary routine share no mutexes, counters, or other shared state, so no two tasks contend for a resource.",
            "Each auxiliary call runs to completion before the calling task starts the next call.",
            "Every schedule and interleaving of the tasks must terminate.",
            "The program finishes with exactly one final status line.",
        ],
        "unverifiable": [1, 5],
        "clauses": {
            "properties": [["R2", "R3", "R4"]],
            "preserved": [],
        },
    },
    "structure/nested_scope_lock_order": {
        "requirements": [
            "A main task starts one outer worker.",
            "The outer worker starts a nested group of two inner tasks.",
            "Each inner task takes mutex A and mutex B and, at some point, holds both of them at the same time.",
            "The two inner tasks take the mutexes in the same order so that no wait cycle can form.",
            "The outer worker completes, and it only completes after both inner tasks have finished.",
            "Every schedule and interleaving of the tasks must terminate.",
            "The program finishes with exactly one final status line.",
        ],
        "unverifiable": [1, 2, 7],
        "clauses": {
            "properties": [["R4", "R6"]],
            "preserved": [["R5"], ["R3"], ["R3"]],
        },
    },
    "structure/scope_bound_k_workers": {
        "requirements": [
            "A main task starts three worker roles that share a single permit, and each role may have up to two activations running at once.",
            "Each activation holds the single permit while it does its work and releases it afterwards.",
            "At most one activation may hold the permit at any moment.",
            "An activation that cannot obtain the permit waits until the permit becomes available.",
            "Every schedule and interleaving must terminate.",
            "All three worker roles complete.",
            "The program finishes with exactly one final status line.",
        ],
        "unverifiable": [1, 2, 7],
        "clauses": {
            "properties": [["R3", "R4", "R5"]],
            "preserved": [["R6"], ["R6"], ["R6"]],
        },
    },
    "structure/scope_worker_abba": {
        "requirements": [
            "A main task starts a group of two workers that both contend for the same two mutexes, A and B.",
            "Each worker takes mutex A and mutex B and, at some point, holds both of them at the same time.",
            "Each worker releases each mutex it took once its work is finished.",
            "At most one worker may hold a given mutex at any moment.",
            "The two workers take the mutexes in the same order so that no wait cycle can form.",
            "A worker that cannot take a held mutex waits until it becomes free.",
            "Every schedule and interleaving of the workers must terminate.",
            "Each worker completes, and the group finishes only after both workers have completed.",
            "The program finishes with exactly one final status line.",
        ],
        "unverifiable": [1, 3, 9],
        "clauses": {
            "properties": [["R4", "R5", "R6", "R7"]],
            "preserved": [["R8"], ["R8"], ["R2"], ["R2"]],
        },
    },
    "structure/spawn_join_loop_finite": {
        "requirements": [
            "A main task starts a worker task and waits for that worker to finish, then begins the same start-and-wait cycle again.",
            "The worker task performs no shared work and shares no mutexes or counters with the main task.",
            "Each started worker is waited for exactly once before the next worker is started.",
            "No task ever waits for a worker that cannot finish, so no execution stalls.",
            "Every schedule and interleaving must terminate.",
            "The program finishes with exactly one final status line.",
        ],
        "unverifiable": [1, 2, 6],
        "clauses": {
            "properties": [["R3", "R4", "R5"]],
            "preserved": [],
        },
    },
    "structure/worker_with_payload": {
        "requirements": [
            "A main task starts a group of two workers.",
            "Each worker takes a shared mutex, calls a sequential helper routine that performs only local computation, then updates a shared counter while still holding the mutex.",
            "Each worker releases the mutex it took before it finishes.",
            "At most one worker may hold the shared mutex at any moment.",
            "A worker that cannot take a held mutex waits until it becomes free.",
            "Every schedule and interleaving of the workers must terminate.",
            "Both workers complete.",
            "The program finishes with exactly one final status line.",
        ],
        "unverifiable": [1, 2, 3, 8],
        "clauses": {
            "properties": [["R4", "R5", "R6"]],
            "preserved": [["R7"], ["R7"]],
        },
    },
}


# The six capability families that make up the generation benchmark.  The
# benchmark targets 24 tasks (all ready cases in these families).
GENERATION_FAMILIES = ("lock-order", "condvar", "channel", "semaphore",
                       "atomic-data", "structure")
