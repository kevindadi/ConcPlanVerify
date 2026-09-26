use std::sync::{Arc, Mutex};

fn main() {
    let a = Arc::new(Mutex::new(()));
    let b = Arc::new(Mutex::new(()));

    let a1 = Arc::clone(&a);
    let b1 = Arc::clone(&b);
    let w1 = std::thread::spawn(move || {
        let _guard_a = a1.lock().unwrap();
        let _guard_b = b1.lock().unwrap();
        // Both held here; drop order is reverse of acquisition: b then a
    });

    let a2 = Arc::clone(&a);
    let b2 = Arc::clone(&b);
    let w2 = std::thread::spawn(move || {
        let _guard_a = a2.lock().unwrap();
        let _guard_b = b2.lock().unwrap();
        // Both held here; drop order is reverse of acquisition: b then a
    });

    w1.join().unwrap();
    w2.join().unwrap();

    println!("DONE done=1");
}
