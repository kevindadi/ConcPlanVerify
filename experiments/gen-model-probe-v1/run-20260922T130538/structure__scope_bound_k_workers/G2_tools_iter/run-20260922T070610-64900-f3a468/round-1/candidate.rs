use std::sync::{Arc, Mutex};
use std::thread;

fn main() {
    // The single shared permit. A Mutex<()> acts as a 1-permit semaphore:
    // at most one activation may hold it at any moment (R3), and lock()
    // blocks until it becomes available (R4).
    let permit = Arc::new(Mutex::new(()));

    // R1: start three worker roles.
    let mut roles = Vec::new();
    for _role in 0..3 {
        let permit = Arc::clone(&permit);
        roles.push(thread::spawn(move || {
            // Each role has up to two activations running at once.
            let mut activations = Vec::new();
            for _ in 0..2 {
                let permit = Arc::clone(&permit);
                activations.push(thread::spawn(move || {
                    // R2/R4: wait for the permit, then hold it during work.
                    let _guard = permit.lock().unwrap();
                    // ... activation work happens while holding the permit ...
                    // R2: the guard is dropped here, releasing the permit.
                }));
            }
            for activation in activations {
                activation.join().unwrap();
            }
        }));
    }

    // R6: wait for all three roles to complete.
    for role in roles {
        role.join().unwrap();
    }

    // R7: print exactly this line, then exit.
    println!("DONE done=1");
}
