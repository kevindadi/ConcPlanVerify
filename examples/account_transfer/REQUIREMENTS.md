# bank_transfer

Concurrent bidirectional bank account transfers with atomicity, consistent balances, per-transaction receipt numbers, and deadlock freedom.

## Entities
- **Roles / Functions**: `transfer_ab`, `transfer_ba`
- **Resources**: `account_a_lock` (Mutex<Int[0..=100]>, init=100), `account_b_lock` (Mutex<Int[0..=100]>, init=100), `total_balance` (Var: Int[0..=200], init=200), `receipt_counter` (Var: Int[0..=2], init=0)
- **Domain Types**: `Receipt` (hole: Pure receipt number and transfer amount fields without synchronization or strings)

## Requirements
- **R1.** [V] (safety) The system must never deadlock under concurrent transfer_ab and transfer_ba execution.
- **R2.** [V] (safety) At all reachable states, total_balance remains equal to 200 (total_balance == 200).
- **R3.** [V] (safety) At all reachable states, receipt_counter stays within 0..=2 (receipt_counter <= 2).
- **R4.** [V] (liveness) Whenever a transfer is initiated, on all execution paths receipt_counter eventually increases by one.
- **R5.** [V] (reachability) Both transfer_ab and transfer_ba workers must be able to complete execution.
- **R6.** [U] (sequential_hole) Construct, inspect, and update Receipt entries and pure integer receipt numbers inside sequential/type holes without synchronization, loops, unsafe, or String formatting.
