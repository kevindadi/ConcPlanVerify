use std::sync::{Arc, Mutex, PoisonError};

fn main() {
    let a = Arc::new(Mutex::new(()));
    let b = Arc::new(Mutex::new(()));

    let a1 = Arc::clone(&a);
    let b1 = Arc::clone(&b);
    let t1 = std::thread::spawn(move || {
        // Fixed global order a then b => no circular wait (R4, R5)
        let _a = a1.lock().unwrap_or_else(PoisonError::into_inner);
        let _b = b1.lock().unwrap_or_else(PoisonError::into_inner);
        // Critical work: both locks held (R3); guards drop, releasing them (R7)
    });

    let a2 = Arc::clone(&a);
    let b2 = Arc::clone(&b);
    let t2 = std::thread::spawn(move || {
        // Same lock order a then b (R5, R8)
        let _a = a2.lock().unwrap_or_else(PoisonError::into_inner);
        let _b = b2.lock().unwrap_or_else(PoisonError::into_inner);
        // Critical work with both locks held (R3)
    });

    t1.join().unwrap_or_else(PoisonError::into_inner).unwrap_or(());
    // simpler and equivalent: consume joins without panicking (R6, R8)
    let _ = t2.join();

    println!("DONE t1=1 t2=1");
}
