# bidirectional_account_transfer

Two threads transfer funds in opposite directions between two mutex-protected bank accounts, incrementing a completed-transfers counter and computing a local receipt number, with deadlock freedom and completion guarantees.

## Entities
- **Roles / Functions**: `transfer_ab`, `transfer_ba`
- **Resources**: `account_a_mutex`, `account_b_mutex`, `completed_transfers`
- **Domain Types**: `Account` (struct: Bank account with balance and pure deposit/withdraw methods)

## Requirements
- **R1.** [V] (safety) The system must never deadlock.
- **R2.** [V] (reachability) Both transfer threads must eventually finish.
- **R3.** [V] (safety) Each transfer locks both accounts at the same time (atomic acquisition of both mutexes).
- **R4.** [V] (safety) Each completed transfer increments the completed-transfers counter.
- **R5.** [U] (sequential_hole) Account stores balance and supports pure deposit/withdraw operations.
- **R6.** [U] (sequential_hole) Each transfer computes a simple receipt number locally.
