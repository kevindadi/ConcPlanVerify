use std::sync::Mutex;

fn main() {
    // Shared locks: first pair (a, b) and second pair (c, d). (R2)
    let a = Mutex::new(());
    let b = Mutex::new(());
    let c = Mutex::new(());
    let d = Mutex::new(());

    // Main thread starts all four workers and waits for them. (R1, R10)
    std::thread::scope(|s| {
        // t1 and t2 both need a and b (R3), both lock in the same
        // relative order a -> b (R6), hold both while working (R4),
        // and release both via guard drop before finishing (R9).
        let t1 = s.spawn(|| {
            let _g1 = a.lock().unwrap();
            let _g2 = b.lock().unwrap();
            // work while holding both locks of the first pair
        });

        let t2 = s.spawn(|| {
            let _g1 = a.lock().unwrap();
            let _g2 = b.lock().unwrap();
            // work while holding both locks of the first pair
        });

        // t3 and t4 both need c and d (R3), both lock in the same
        // relative order c -> d (R7). These locks are disjoint from
        // a and b, so this pair is independent of the first (R5).
        let t3 = s.spawn(|| {
            let _g1 = c.lock().unwrap();
            let _g2 = d.lock().unwrap();
            // work while holding both locks of the second pair
        });

        let t4 = s.spawn(|| {
            let _g1 = c.lock().unwrap();
            let _g2 = d.lock().unwrap();
            // work while holding both locks of the second pair
        });

        // Main thread finishes only after all four workers finish. (R10)
        t1.join().unwrap();
        t2.join().unwrap();
        t3.join().unwrap();
        t4.join().unwrap();
    });

    // Exactly one line of output, then exit. (R12)
    println!("DONE done=1");
}
