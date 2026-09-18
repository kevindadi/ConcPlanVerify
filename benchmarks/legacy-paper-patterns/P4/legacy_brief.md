# P4 legacy brief (three_way_deadlock)

Canonical requirement from the legacy corpus (draft; may reveal the
defect and therefore is NOT used as the live generation input):

Model a program with three mutexes A, B, C and three worker threads. Worker 1 needs A and B, worker 2 needs B and C, worker 3 needs C and A. Each worker holds its two mutexes at the same time, then releases them. Main spawns all three workers and joins them. Every interleaving must terminate.
