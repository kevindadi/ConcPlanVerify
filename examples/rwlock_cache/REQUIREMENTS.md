# concurrent_cache_rwlock

A small in-memory cache guarded by an RwLock with one writer thread recording and clearing updates and one reader thread inspecting cached data concurrently.

## Entities
- **Roles / Functions**: `writer`, `reader`
- **Resources**: `cache_lock`, `pending_updates`
- **Domain Types**: `CacheEntry` (struct: In-memory cache entry struct holding cached payload data and pure accessor/update methods)

## Requirements
- **R1.** [V] (safety) The system must never deadlock.
- **R2.** [V] (safety) The writer thread acquires the write lock to record an update.
- **R3.** [V] (safety) The writer thread later acquires the write lock to clear pending updates.
- **R4.** [V] (safety) The reader thread acquires the read lock to inspect cached data concurrently with other readers.
- **R5.** [U] (sequential_hole) CacheEntry stores cached payload data and provides pure accessor/update methods in memory.
- **R6.** [V] (reachability) Both the writer and reader threads must eventually complete.
