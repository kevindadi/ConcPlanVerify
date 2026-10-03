# P7 legacy brief (semaphore_throttle)

Canonical requirement from the legacy corpus (draft; may reveal the
defect and therefore is NOT used as the live generation input):

Model three worker threads throttled by a counting semaphore with two permits. Each worker acquires one permit, does its work, releases the permit, and returns. Main spawns all three workers and joins them. All interleavings must terminate.
