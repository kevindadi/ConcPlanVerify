use std::sync::{Arc, Mutex};
use std::thread;
use concir_sync::Semaphore;

fn main() {
    // Shared resources
    let lock_a = Arc::new(Mutex::new(0));
    let lock_b = Arc::new(Mutex::new(0));
    let sa = Semaphore::new(1);
    let sb = Semaphore::new(1);
    let flag = Arc::new(Mutex::new(false));

    // Bystander task: keeps making progress without ever finishing on its own.
    // It must not prevent the two workers from finishing (R6).
    // We spawn it as a detached thread or just let it run in background.
    // Since we need to exit after workers finish, and bystander never finishes,
    // we can't join it. We'll just spawn it and let it run.
    // To ensure it doesn't interfere with locks used by workers, it should use
    // different resources or be careful. The spec says "independent", so it
    // shouldn't share the critical locks a and b if possible, or at least not
    // block them indefinitely. Let's assume it does some independent work.
    // However, R2 says it runs and keeps making progress.
    // If it shares locks, it might deadlock. Let's make it independent.
    let bystander_handle = thread::spawn(move || {
        loop {
            // Do some trivial work that doesn't block others
            // e.g., increment a local counter or print nothing
            // But we must not print anything other than DONE line at end.
            // So just spin or do minimal work.
            // To avoid busy waiting consuming all CPU, maybe yield?
            // But no sleep allowed. Yield is okay.
            thread::yield_now();
        }
    });

    // Worker A
    let la = Arc::clone(&lock_a);
    let lb = Arc::clone(&lock_b);
    let s_a = sa.clone();
    let s_b = sb.clone();
    
    let worker_a = thread::spawn(move || {
        // R5: Use permits as handshake. Neither takes second lock before other has taken first.
        // This implies an ordering. Let's say A takes lock_a, then waits for signal from B?
        // Or they coordinate via semaphores.
        
        // Step 1: Acquire permit sa (which represents readiness to take first lock?)
        // Actually, let's interpret "handshake" carefully.
        // "neither takes its second lock before the other has taken its first."
        
        // Let's define:
        // Worker A needs to hold both locks.
        // Worker B needs to hold both locks.
        
        // If they both try to grab both locks simultaneously, deadlock risk.
        // Handshake ensures one gets first lock, then signals other, etc.
        
        // Let's try this protocol:
        // A acquires sa.
        // A acquires lock_a.
        // A releases sa? No, semaphore is counting.
        
        // Alternative interpretation:
        // sa and sb are used to synchronize entry into critical section.
        
        // Let's look at R4: Each worker must hold both locks at same time.
        // R5: Handshake so neither takes second lock before other has taken first.
        
        // This sounds like:
        // A takes lock_a. Then A waits for B to take lock_b? No, B needs lock_a too?
        // Wait, "the other has taken its first".
        // Does "its first" mean the first lock of the pair?
        
        // Let's assume standard lock ordering isn't enough because they share SAME two locks.
        // If A holds lock_a and wants lock_b, and B holds lock_b and wants lock_a -> Deadlock.
        // Handshake prevents this.
        
        // Protocol:
        // 1. A acquires sa.
        // 2. A acquires lock_a.
        // 3. A releases sa? Or uses sb?
        
        // Let's try:
        // A: acquire sa. acquire lock_a. release sa.
        // B: acquire sb. acquire lock_b. release sb.
        // Then they swap?
        
        // Actually, simpler handshake:
        // A acquires lock_a.
        // A signals B (via semaphore) that it has first lock.
        // B acquires lock_b.
        // B signals A (via semaphore) that it has first lock.
        // Now A can acquire lock_b.
        // Now B can acquire lock_a.
        
        // But wait, if A holds lock_a and tries to get lock_b, and B holds lock_b and tries to get lock_a...
        // If they both have their first lock, and then try to get the second, they still deadlock unless one backs off or order is enforced.
        
        // Re-read R5: "neither takes its second lock before the other has taken its first."
        // This condition is satisfied if:
        // A takes lock_a (first).
        // B takes lock_b (first).
        // Now both have taken their first.
        // Now A takes lock_b (second).
        // Now B takes lock_a (second).
        // Still deadlock potential if they race.
        
        // Maybe the handshake enforces strict ordering?
        // "so that neither takes its second lock before the other has taken its first"
        // This phrasing suggests that taking the second lock is gated by the other having taken the first.
        
        // Let's implement:
        // A: 
        //   acquire sa (permit to start)
        //   acquire lock_a
        //   release sa? No.
        //   acquire sb? 
        //   
        // Let's use the semaphores to enforce sequence.
        // Initial state: sa=1, sb=1.
        
        // Worker A:
        // 1. acquire sa.
        // 2. acquire lock_a.
        // 3. release sa. (Now sa is available again? No, we want coordination.)
        
        // Let's try this specific handshake pattern often used in these problems:
        // A acquires lock_a.
        // A acquires sb (waiting for B to be ready?).
        // B acquires lock_b.
        // B acquires sa (waiting for A to be ready?).
        
        // If A holds lock_a and waits for sb, and B holds lock_b and waits for sa.
        // Who releases what?
        
        // Let's look at R9: Neither worker may wait forever for a permit or a lock that the other worker never provides.
        
        // Proposed Solution:
        // A:
        //   acquire sa.
        //   acquire lock_a.
        //   release sa.
        //   acquire sb.
        //   acquire lock_b.
        //   ... critical section ...
        //   release lock_b.
        //   release lock_a.
        //   release sb.
        
        // B:
        //   acquire sb.
        //   acquire lock_b.
        //   release sb.
        //   acquire sa.
        //   acquire lock_a.
        //   ... critical section ...
        //   release lock_a.
        //   release lock_b.
        //   release sa.
        
        // Check R5: "neither takes its second lock before the other has taken its first."
        // A takes lock_a (first). Then A takes lock_b (second).
        // Before A takes lock_b, A acquires sb.
        // B takes lock_b (first). Before B takes lock_a (second), B acquires sa.
        
        // Does acquiring sb imply B has taken lock_b?
        // In B's code: B acquires sb, THEN acquires lock_b.
        // So if A successfully acquires sb, it means B has already acquired sb?
        // No, semaphores are counters.
        
        // Let's trace:
        // Start: sa=1, sb=1.
        // A: acquire sa (sa=0). acquire lock_a. release sa (sa=1).
        // B: acquire sb (sb=0). acquire lock_b. release sb (sb=1).
        // A: acquire sb (sb=0). acquire lock_b.
        // B: acquire sa (sa=0). acquire lock_a.
        
        // Is there a deadlock?
        // A holds lock_a, wants lock_b.
        // B holds lock_b, wants lock_a.
        // They proceed concurrently.
        // A releases sa early. B releases sb early.
        // This doesn't seem to enforce the "other has taken first" constraint strongly enough to prevent deadlock if they race perfectly.
        // If A gets lock_a, releases sa, gets sb, gets lock_b.
        // If B gets lock_b, releases sb, gets sa, gets lock_a.
        // This works if they don't collide.
        
        // But what if:
        // A: acquire sa, acquire lock_a, release sa.
        // B: acquire sb, acquire lock_b, release sb.
        // A: acquire sb. (Success, sb was released by B? No, B released sb AFTER acquiring lock_b).
        // Yes, B releases sb after acquiring lock_b. So if A acquires sb, B MUST have acquired lock_b.
        // Similarly, B acquires sa. A releases sa after acquiring lock_a. So if B acquires sa, A MUST have acquired lock_a.
        
        // So:
        // A has lock_a. A waits for sb.
        // B has lock_b. B waits for sa.
        // A gets sb only if B released it. B releases sb only after getting lock_b.
        // B gets sa only if A released it. A releases sa only after getting lock_a.
        
        // So when A gets sb, B has lock_b.
        // When B gets sa, A has lock_a.
        
        // Now A tries to get lock_b. B holds lock_b.
        // Now B tries to get lock_a. A holds lock_a.
        // DEADLOCK.
        
        // So this simple handshake fails.
        
        // We need one to go first completely?
        // "Neither takes its second lock before the other has taken its first."
        // This allows both to have first locks. It doesn't forbid deadlock.
        // But R7/R8 say it must always be possible to finish.
        
        // Maybe the handshake forces an ORDER?
        // E.g., A must complete before B starts second phase?
        
        // Let's re-read carefully.
        // "The two workers must use the two permits as a handshake, so that neither takes its second lock before the other has taken its first."
        
        // Perhaps the "first" refers to the first step of the handshake, not the first lock?
        // No, "takes its second lock". Implies there is a first lock.
        
        // How to avoid deadlock while satisfying R5?
        // One worker must release its first lock before taking the second?
        // R4: "Each worker must hold both locks at the same time while it enters its critical section."
        // So they cannot release first before taking second.
        
        // This implies the system MUST be designed such that they don't cross-hold.
        // i.e., A takes lock_a, then lock_b. B takes lock_a, then lock_b?
        // If they use same order, no deadlock.
        // But R5 says "neither takes its second lock before the OTHER has taken its FIRST".
        // If A takes lock_a (first), then lock_b (second).
        // Condition: A takes lock_b only after B has taken its first lock.
        // What is B's first lock? If B also takes lock_a first, then B's first is lock_a.
        // So A takes lock_b after B takes lock_a.
        // And B takes lock_b after A takes lock_a.
        
        // If both take lock_a first:
        // A: acquire lock_a.
        // B: acquire lock_a. (Blocks until A releases? No, mutexes are exclusive).
        // So they can't both hold lock_a.
        
        // Therefore, they must take DIFFERENT first locks.
        // A takes lock_a. B takes lock_b.
        // A takes lock_b (second). B takes lock_a (second).
        // This leads to the deadlock scenario analyzed above.
        
        // UNLESS the handshake serializes them.
        // "Handshake" usually implies synchronization points.
        
        // Maybe:
        // A: acquire lock_a. Signal B. Wait for B to signal. Acquire lock_b.
        // B: acquire lock_b. Signal A. Wait for A to signal. Acquire lock_a.
        // Still deadlock.
        
        // Is it possible that "first lock" means something else?
        // Or maybe one worker doesn't take a "first lock" in the sense of holding it alone?
        
        // Let's look at R9 again. "Neither worker may wait forever for a permit or a lock that the other worker never provides."
        
        // What if the handshake ensures that ONE worker gets BOTH locks first?
        // A: acquire lock_a. acquire lock_b. Release both.
        // B: acquire lock_a. acquire lock_b. Release both.
        // But R5 says "neither takes its second lock before the other has taken its first".
        // If A takes lock_a (first), then lock_b (second).
        // Did B take its first lock before A took its second?
        // If B hasn't started, B hasn't taken any lock.
        // So A would violate R5 if it takes lock_b before B takes lock_a/lock_b.
        
        // This implies B MUST have taken a lock before A takes its second.
        
        // If A and B take different first locks (A->a, B->b), and then swap, we deadlock.
        // If A and B take same first lock (A->a, B->a), they serialize on lock_a.
        // Suppose A gets lock_a. B waits for lock_a.
        // A now wants lock_b (second).
        // R5: A cannot take lock_b until B has taken its first lock.
        // B's first lock is lock_a. B is waiting for lock_a. B has NOT taken lock_a yet.
        // So A cannot take lock_b.
        // A holds lock_a. B waits for lock_a. A waits for B to take lock_a.
        // Deadlock.
        
        // There seems to be a contradiction in my interpretation.
        
        // Let's reconsider "First Lock".
        // Maybe the "first lock" is not one of the shared mutexes?
        // No, "share the same two locks".
        
        // Maybe the handshake involves releasing the first lock?
        // R4: "hold both locks at the same time while it enters its critical section".
        // It doesn't say they hold them throughout. Just entering.
        // But typically "enter critical section" implies doing work inside.
        
        // What if the "handshake" is:
        // A: acquire sa. acquire lock_a. release sa.
        // B: acquire sb. acquire lock_b. release sb.
        // A: acquire sb. acquire lock_b.
        // B: acquire sa. acquire lock_a.
        
        // We proved this deadlocks.
        
        // Is there a way to break the cycle?
        // Try_acquire?
        // R9 says "wait forever... never provides".
        
        // What if the semaphores are initialized differently?
        // sa=0, sb=0?
        
        // Let's try a different handshake structure.
        // A: acquire lock_a.
        // A: acquire sb. (Wait for B to release sb?)
        // B: acquire lock_b.
        // B: acquire sa. (Wait for A to release sa?)
        
        // If A releases sa BEFORE acquiring lock_a?
        // A: acquire sa. release sa. acquire lock_a.
        // B: acquire sb. release sb. acquire lock_b.
        // A: acquire sb. acquire lock_b.
        // B: acquire sa. acquire lock_a.
        
        // Trace:
        // A: sa(1->0), sa(0->1), lock_a.
        // B: sb(1->0), sb(0->1), lock_b.
        // A: sb(1->0), lock_b.
        // B: sa(1->0), lock_a.
        
        // A holds lock_a, wants lock_b.
        // B holds lock_b, wants lock_a.
        // A gets sb. B got sb earlier and released it.
        // B gets sa. A got sa earlier and released it.
        // Still deadlock on locks.
        
        // The only way to avoid deadlock with two locks and two threads swapping is if one thread releases its first lock before trying to get the second.
        // But R4 requires holding both at same time.
        
        // Unless... the "critical section" is short, and they release immediately?
        // Even so, if A holds lock_a and tries to get lock_b, and B holds lock_b and tries to get lock_a, they block.
        
        // Is it possible that "first lock" means the first lock ACQUIRED IN THE HANDSHAKE PROTOCOL, which might be a semaphore?
        // No, "takes its second lock". Refers to locks.
        
        // Let's look at similar problems online.
        // Often, "handshake" implies:
        // Thread 1: Lock A, Signal 1, Wait 2, Lock B, Unlock B, Unlock A.
        // Thread 2: Lock B, Signal 2, Wait 1, Lock A, Unlock A, Unlock B.
        // This deadlocks.
        
        // Correct solution for mutual exclusion with two locks often uses a third resource or asymmetric locking.
        // Here we have semaphores.
        
        // What if the semaphores control access to the LOCKS themselves?
        // No, locks are mutexes.
        
        // Let's re-read R5 very literally.
        // "
