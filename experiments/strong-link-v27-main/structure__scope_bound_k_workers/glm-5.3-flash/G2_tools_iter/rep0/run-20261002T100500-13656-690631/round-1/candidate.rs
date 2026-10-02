// Roles: w1, w2, w3. Shared resource: s (single permit).
use std::sync::{Arc, Mutex};
use std::thread;

// One activation of a worker role.
// R2: holds the permit while doing its work, releases it afterwards
//     (released automatically when `_permit` is dropped).
// R4: if the permit is unavailable, `lock()` waits until it is free.
fn activation(role: &'static str, n: u32, s: Arc<Mutex<()>>) {
    let _permit = s.lock().unwrap(); // acquire the single permit (or wait)
    // Do the activation's work while holding the permit.
    // No output here: R7 requires exactly one printed line.
    let _ = (role, n);
} // permit released here

fn main() {
    // s: the single shared permit.
    // R3: a Mutex guarantees at most one holder at any moment.
    let s = Arc::new(Mutex::new(()));
    let mut handles = Vec::new();

    // R1: start the three worker roles w1, w2, w3;
    // each role runs up to two activations concurrently.
    for role in ["w1", "w2", "w3"] {
        for n in 0..2 {
            let s = Arc::clone(&s);
            handles.push(thread::spawn(move || activation(role, n, s)));
        }
    }

    // R5/R6: join every activation so all roles complete and
    // every interleaving terminates before printing the result.
    for h in handles {
        h.join().unwrap();
    }

    // R7: print exactly this line, then exit.
    println!("DONE done=1");
}
