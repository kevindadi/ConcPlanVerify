use std::sync::{Arc, Condvar, Mutex};
use std::thread;

fn main() {
    let pair = Arc::new((Mutex::new((0usize, false)), Condvar::new()));
    let ready = Arc::new((Mutex::new(0usize), Condvar::new()));

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

            let (lock, cvar) = &*pair;
            let mut guard = lock.lock().unwrap();
            while !guard.1 {
                guard = cvar.wait(guard).unwrap();
            }
            guard.0 += 1;
        }));
    }
    ...
