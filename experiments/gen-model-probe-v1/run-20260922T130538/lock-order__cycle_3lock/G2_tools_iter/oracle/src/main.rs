mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

fn main() { cir_trace::init();
    // Three locks shared among the workers.
    let locks = [
        Arc::new(Mutex::new_named("locks_mutex0", ())),
        Arc::new(Mutex::new_named("locks_mutex1", ())),
        Arc::new(Mutex::new_named("locks_mutex2", ())),
    ];

    let mut handles = Vec::new();

    for i in 0..3 {
        let locks = locks.clone();
        handles.push(thread::spawn(move || {
            // Worker i needs locks i and (i + 1) % 3:
            //   worker 0 -> locks 0 and 1
            //   worker 1 -> locks 1 and 2
            //   worker 2 -> locks 2 and 0
            let a = i;
            let b = (i + 1) % 3;

            // Always acquire the lower-indexed lock first. This global
            // ordering breaks any circular wait, so deadlock is impossible
            // and every schedule terminates.
            let (first, second) = if a < b { (a, b) } else { (b, a) };

            let guard1 = locks[first].lock().unwrap();
            let guard2 = locks[second].lock().unwrap();

            // Critical work: both locks are held simultaneously here.
            let _hold_both = (&guard1, &guard2);

            // Guards are dropped here, releasing both locks before the
            // worker finishes.
        }));
    }

    // The main thread finishes only after all workers have finished.
    for handle in handles {
        handle.join().unwrap();
    }

    println!("DONE done=1");
 cir_trace::finish();}
