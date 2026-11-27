# single_item_producer_consumer

Single-item producer-consumer handoff using a mutex and condition variable, with producer marking ready and waking consumer, consumer computing checksum, ensuring no deadlock/lost wakeup and eventual consumption.

## Entities
- **Roles / Functions**: `producer`, `consumer`
- **Resources**: `mutex`, `condvar`, `slot`, `ready`
- **Domain Types**: `Checksum` (hole: Simple checksum computation over the consumed integer item)

## Requirements
- **R1.** [V] (safety) The system must never deadlock.
- **R2.** [V] (safety) There must be no lost wakeup: if the producer marks the slot ready and signals, the consumer must not miss the notification.
- **R3.** [V] (liveness) Any produced item is eventually consumed.
- **R4.** [V] (reachability) Both producer and consumer threads eventually exit (complete).
- **R5.** [U] (sequential_hole) The consumer computes a simple checksum over the consumed integer item.
