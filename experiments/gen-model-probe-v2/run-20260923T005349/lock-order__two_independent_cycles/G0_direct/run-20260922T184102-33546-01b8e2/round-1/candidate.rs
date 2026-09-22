use std::sync::{Arc, Mutex};
use std::thread;

fn main() {
    // Four shared locks, arranged as a first pair (a, b) and a second pair (c, d).
    let a = Arc::new(Mutex::new(0u64));
    let b = Arc::new(Mutex::new(0u64));
    let c = Arc::new(Mutex::new(0u64));
    let d = Arc::new(Mutex::new(0u64));

    let mut handles = Vec::new();

    // t1: needs both locks of the first pair, acquired in the order a then b.
    {
        let a = Arc::clone(&a);
        let b = Arc::clone(&b);
        handles.push(
            thread::Builder::new()
                .name("t1".to_string())
                .spawn(move || {
                    let mut guard_a = a.lock().unwrap();
                    let mut guard_b = b.lock().unwrap();
                    // Work while holding both locks of the pair simultaneously.
                    *guard_a += 1;
                    *guard_b += 1;
                    // Release both locks before finishing.
                    drop(guard_b);
                    drop(guard_a);
                })
                .unwrap(),
        );
    }

    // t2: needs both locks of the first pair, acquired in the same order a then b.
    {
        let a = Arc::clone(&a);
        let b = Arc::clone(&b);
        handles.push(
            thread::Builder::new()
                .name("t2".to_string())
                .spawn(move || {
                    let mut guard_a = a.lock().unwrap();
                    let mut guard_b = b.lock().unwrap();
                    // Work while holding both locks of the pair simultaneously.
                    *guard_a += 1;
                    *guard_b += 1;
                    // Release both locks before finishing.
                    drop(guard_b);
                    drop(guard_a);
                })
                .unwrap(),
        );
    }

    // t3: needs both locks of the second pair, acquired in the order c then d.
    {
        let c = Arc::clone(&c);
        let d = Arc::clone(&d);
        handles.push(
            thread::Builder::new()
                .name("t3".to_string())
                .spawn(move || {
                    let mut guard_c = c.lock().unwrap();
                    let mut guard_d = d.lock().unwrap();
                    // Work while holding both locks of the pair simultaneously.
                    *guard_c += 1;
                    *guard_d += 1;
                    // Release both locks before finishing.
                    drop(guard_d);
                    drop(guard_c);
                })
                .unwrap(),
        );
    }

    // t4: needs both locks of the second pair, acquired in the same order c then d.
    {
        let c = Arc::clone(&c);
        let d = Arc::clone(&d);
        handles.push(
            thread::Builder::new()
                .name("t4".to_string())
                .spawn(move || {
                    let mut guard_c = c.lock().unwrap();
                    let mut guard_d = d.lock().unwrap();
                    // Work while holding both locks of the pair simultaneously.
                    *guard_c += 1;
                    *guard_d += 1;
                    // Release both locks before finishing.
                    drop(guard_d);
                    drop(guard_c);
                })
                .unwrap(),
        );
    }

    // The main thread only finishes after all four workers have finished.
    for handle in handles {
        handle.join().unwrap();
    }

    println!("DONE done=1");
}
