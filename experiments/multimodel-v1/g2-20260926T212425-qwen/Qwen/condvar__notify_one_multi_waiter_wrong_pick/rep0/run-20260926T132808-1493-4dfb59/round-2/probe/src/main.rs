use std::sync::{Arc, Mutex, Condvar};
use std::thread;
use concir_sync::Semaphore;

fn main() {
    // Shared state: a lock and a condition variable.
    // We use a boolean flag to indicate whether the waiters should proceed.
    let pair = Arc::new((Mutex::new(false), Condvar::new()));
    
    // g12 is used by waiters to signal they are ready to wait (or have started).
    // The spec says "separate permit counter so that the notifier only wakes the waiters after both are ready to wait".
    // This implies we need to know when both w1 and w2 are blocked/waiting.
    // However, standard CV usage doesn't easily expose "is waiting" without extra state.
    // Let's interpret R6 carefully: "The waiters and the notifier use a separate permit counter...".
    // Strategy:
    // 1. Waiters acquire a permit from g12 before waiting? Or release it?
    //    If g12 starts with 0 permits, and each waiter releases one when they are about to wait,
    //    then the notifier can acquire two permits from g12 to ensure both are ready.
    //    But `acquire` blocks if no permits. So if g12 has 0, acquire blocks.
    //    If waiters *release* into g12, then g12 count increases.
    //    Notifier acquires 2 from g12. This ensures 2 waiters have signaled readiness.
    
    // 2. What is gN? Maybe unused or for other sync? The prompt lists g12 and gN as shared resources.
    //    Perhaps gN is not strictly needed for the core logic if g12 handles the readiness, but I must declare/use them if implied.
    //    Actually, looking at typical patterns, maybe gN is for the notifier to signal completion?
    //    But R10 requires printing DONE waiters=0.
    
    // Let's refine the synchronization:
    // - `pair`: Mutex<bool> + Condvar. The bool indicates if it's time to wake up.
    // - `g12`: Semaphore initialized to 0. Waiters will `release()` this semaphore once they are inside the critical section and about to wait (or just before waiting) to signal "I am here".
    //   Wait, if they release before waiting, there's a race where notifier sees them ready but they haven't actually called `wait` yet.
    //   Standard pattern: 
    //     Waiter: Lock -> Check Condition -> If false, Release g12 (signal presence) -> Wait on CV -> ...
    //     Notifier: Acquire g12 twice (ensure 2 present) -> Lock -> Set Condition True -> Notify All -> Unlock.
    //   This seems robust enough for the constraints.
    
    // - `gN`: The prompt mentions it. Maybe it's not used in the final logic but must exist? Or maybe it's used for the final join?
    //   Let's assume gN is not strictly required for the wake-up logic described in R6-R7, but since it's listed as an entity, I'll initialize it. 
    //   Actually, often in these problems, if an entity is listed, it might be part of the solution structure. 
    //   Let's look at R6 again: "The waiters and the notifier use a separate permit counter...". Singular "counter" but plural "semaphores" in entities.
    //   Maybe g12 is for the 2 waiters, and gN is for something else? Or maybe gN is unused?
    //   To be safe, I will initialize both. If gN isn't needed for the logic, I won't force its use incorrectly.
    
    let g12 = Semaphore::new(0);
    let _gN = Semaphore::new(0); // Initialized, potentially unused if not needed for specific sub-logic
    
    let pair_clone = Arc::clone(&pair);
    let g12_clone_w1 = Arc::clone(&g12);
    let g12_clone_w2 = Arc::clone(&g12);
    let g12_clone_n = Arc::clone(&g12);
    
    // Start Waiter 1
    let h1 = thread::spawn(move || {
        let (lock, cvar) = &*pair_clone;
        let mut guard = lock.lock().unwrap();
        
        // Signal that this waiter is ready to wait (has acquired lock)
        // Note: Releasing a semaphore increments its count.
        g12_clone_w1.release();
        
        // Wait until notified
        while !*guard {
            guard = cvar.wait(guard).unwrap();
        }
        
        // Done waiting
    });
    
    // Start Waiter 2
    let h2 = thread::spawn(move || {
        let (lock, cvar) = &*pair_clone;
        let mut guard = lock.lock().unwrap();
        
        // Signal that this waiter is ready to wait
        g12_clone_w2.release();
        
        // Wait until notified
        while !*guard {
            guard = cvar.wait(guard).unwrap();
        }
        
        // Done waiting
    });
    
    // Start Notifier
    let hn = thread::spawn(move || {
        // Ensure both waiters are ready (have released their permits into g12)
        // Acquire 2 permits. This blocks until 2 releases happen.
        let p1 = g12_clone_n.acquire();
        let p2 = g12_clone_n.acquire();
        
        // Now we know both waiters have entered the lock and signaled.
        // There is still a tiny race: they might have signaled but not yet called cv.wait().
        // However, they hold the lock. The notifier cannot take the lock until they release it.
        // They release the lock ONLY when they call cv.wait().
        // So, acquiring the lock guarantees they are either waiting or have finished waiting.
        // Since the condition is initially false, they MUST be waiting (or about to wait, but holding lock prevents progress).
        // Actually, if they hold the lock, they haven't called wait yet.
        // Sequence:
        // W1: Lock -> Release g12 -> While(!flag) { Wait }
        // N: Acquire g12 x2 -> Lock -> Set Flag -> NotifyAll -> Unlock
        
        // When N tries to Lock, W1/W2 hold it. N blocks.
        // W1/W2 see flag is false, call Wait. Wait releases the lock.
        // N gets the lock. Sets flag true. Notifies all. Unlocks.
        // W1/W2 wake up, check flag (true), exit loop.
        
        let (lock, cvar) = &*pair_clone;
        let mut guard = lock.lock().unwrap();
        
        *guard = true;
        cvar.notify_all();
        
        // Drop guard to release lock
        drop(guard);
        
        // Permits p1, p2 dropped here, releasing them back to semaphore (not strictly necessary for termination but good practice)
    });
    
    // Join all threads
    h1.join().unwrap();
    h2.join().unwrap();
    hn.join().unwrap();
    
    println!("DONE waiters=0");
}
