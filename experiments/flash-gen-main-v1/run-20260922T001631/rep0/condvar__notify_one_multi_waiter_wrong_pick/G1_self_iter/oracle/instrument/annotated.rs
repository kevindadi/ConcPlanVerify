mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

fn main() { cir_trace::init();
    let pair = Arc::new((Mutex::new_named("pair_mutex0", (0usize, false)), Condvar::new_named("pair_condvar0")));
    let ready = Arc::new((Mutex::new_named("ready_mutex0", 0usize), Condvar::new_named("ready_condvar0")));

    let mut handles = Vec::new();

    for _ in 0..2 {
        let pair = Arc::clone(&pair);
        let ready = Arc::clone(&ready);
        handles.push(thread::spawn(move || {
            // Signal readiness.
            {
                let (lock, cvar) = &*ready;
                let mut count = lock.lock().unwrap();
                *count += 1;
                if *count == 2 {
                    cvar.notify_all();
                }
            }

            // Wait until notifier says go.
            let (lock, cvar) = &*pair;
            let mut guard = lock.lock().unwrap();
            while !guard.1 {
                guard = cvar.wait(guard).unwrap();
            }
        }));
    }

    // Notifier: wait until both waiters are ready, then wake them.
    {
        let ready = Arc::clone(&ready);
        let pair = Arc::clone(&pair);
        handles.push(thread::spawn(move || {
            {
                let (lock, cvar) = &*ready;
                let mut count = lock.lock().unwrap();
                while *count < 2 {
                    count = cvar.wait(count).unwrap();
                }
            }

            let (lock, cvar) = &*pair;
            let mut guard = lock.lock().unwrap();
            guard.1 = true;
            cvar.notify_all();
        }));
    }

    for h in handles {
        h.join().unwrap();
    }

    println!("DONE waiters=0");
 cir_trace::finish();}
