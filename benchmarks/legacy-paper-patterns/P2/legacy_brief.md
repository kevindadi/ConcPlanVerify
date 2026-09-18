# P2 legacy brief (signal_loss)

Canonical requirement from the legacy corpus (draft; may reveal the
defect and therefore is NOT used as the live generation input):

Model a waiter/notifier pair sharing a mutex, a condition variable, and a boolean flag `ready` protected by the mutex. The notifier sets `ready` to true and signals the condition variable. The waiter must block until `ready` is true and then finish. The waiter must terminate even if the notifier signals before the waiter starts waiting.
