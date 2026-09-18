# P1 legacy brief (mutex_deadlock)

Canonical requirement from the legacy corpus (draft; may reveal the
defect and therefore is NOT used as the live generation input):

Model a program with two mutexes A and B and two worker threads. Each worker must acquire both mutexes, do its work inside the critical section, release both mutexes, and terminate. The main thread spawns both workers and joins them. The program must always terminate without deadlock.
