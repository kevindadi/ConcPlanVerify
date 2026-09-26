mod cir_trace;
use std::thread;
use concir_sync::Semaphore;

fn main() { cir_trace::init();
    let s = Semaphore::new_named("s_semaphore0", 2);

    let mut handles = Vec::new();

    for i in 1..=3 {
        let s_clone = s.clone();
        let handle = cir_trace::spawn("handle", move || {
            // Acquire a permit (blocks if none available)
            let permit = s_clone.acquire();
            
            // Perform work (simulated by just holding the permit briefly)
            // In a real scenario, this would be actual computation.
            // The key is that we hold the permit while working.
            
            // Release the permit explicitly or via drop at end of scope
            // Using explicit release to ensure it happens before thread finishes logic
            permit.release();
        });
        handles.push(handle);
    }

    // Wait for all workers to finish
    for h in handles {
        h.join().unwrap();
    }

    println!("DONE done=1");
 cir_trace::finish();}
