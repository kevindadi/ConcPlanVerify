use std::sync::{Arc, Mutex};
use std::thread;

fn main() {
    // Shared resources: mutex `m` protecting variable `c` (init 0).
    let m = Arc::new(Mutex::new(0i32));

    let w1 = {
        let m = Arc::clone(&m);
        move || {
            let mut c = m.lock().unwrap(); // mutex_lock main::m
            if *c < 1 {
                *c += 1; // write_shared c = c + 1
            }
            // mutex_unlock main::m (guard dropped)
        }
    };

    let w2 = {
        let m = Arc::clone(&m);
        move || {
            let mut c = m.lock().unwrap(); // mutex_lock main::m
            if *c < 1 {
                *c += 1; // write_shared c = c + 1
            }
            // mutex_unlock main::m (guard dropped)
        }
    };

    let h1 = thread::spawn(w1);
    let h2 = thread::spawn(w2);

    h1.join().unwrap();
    h2.join().unwrap();

    let done = *m.lock().unwrap();
    println!("DONE done={}", done);
}
