# P3 legacy brief (channel_deadlock)

Canonical requirement from the legacy corpus (draft; may reveal the
defect and therefore is NOT used as the live generation input):

Model a sender and a receiver communicating over a channel, plus one shared mutex both threads occasionally need. The sender sends an integer over the channel; the receiver receives it. Both threads also enter a mutex-guarded critical section. Every interleaving must terminate: the receiver must never block on the channel while holding the mutex the sender needs.
