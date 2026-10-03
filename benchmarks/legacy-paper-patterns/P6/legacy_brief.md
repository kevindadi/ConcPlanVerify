# P6 legacy brief (dual_condvar)

Canonical requirement from the legacy corpus (draft; may reveal the
defect and therefore is NOT used as the live generation input):

Model two threads A and B with two mutexes and two condition variables. Thread A waits on condvar 1 and then notifies condvar 2; thread B waits on condvar 2 and then notifies condvar 1. The design must break the mutual wait so both threads terminate in every interleaving.
