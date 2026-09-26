use concir_sync::Semaphore;
use std::thread;

fn main() {
    // R1: A single permit shared among three worker roles.
    let s = Semaphore::new(1);

    // Helper to spawn a role with up to two activations (R1).
    // Each activation holds the permit while doing work and releases it afterwards (R2).
    // At most one activation may hold the permit at any moment (R3) - enforced by semaphore.
    // An activation that cannot obtain the permit waits until available (R4) - acquire blocks.
    // Every schedule terminates (R5) - finite number of activations, each completes.
    // All three worker roles complete (R6) - we join all threads.
    // Print exactly "DONE done=1" and exit (R7).

    let mut handles = Vec::new();

    for i in 0..3 {
        let sem_clone = s.clone();
        // Spawn two activations per role (R1: up to two activations running at once)
        for _ in 0..2 {
            let sem = sem_clone.clone();
            let handle = thread::spawn(move || {
                // Acquire the permit (blocks if not available, satisfying R4)
                let permit = sem.acquire();
                // Do work while holding the permit (R2)
                // The permit is released when `permit` goes out of scope (Drop)
                drop(permit);
            });
            handles.push(handle);
        }
    }

    // Wait for all activations to complete (ensures R6 and termination R5)
    for h in handles {
        h.join().unwrap();
    }

    println!("DONE done=1");
}
