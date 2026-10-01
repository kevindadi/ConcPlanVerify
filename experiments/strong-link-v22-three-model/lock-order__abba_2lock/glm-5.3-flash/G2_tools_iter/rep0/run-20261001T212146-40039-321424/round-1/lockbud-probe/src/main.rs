use std::sync::{Arc, Mutex};
use std::thread;

fn worker(
    name: &'static str,
    a: Arc<Mutex<()>>,
    b: Arc<Mutex<()>>,
    count: Arc<Mutex<u32>>,
) {
    // R4: blocking lock acquisition (waits until free, then continues).
    // R5: always acquire in the same order (a then b), so no circular wait
    //     is possible and deadlock can never occur (R8).
    {
        let _a = a.lock().unwrap(); // hold a
        let _b = b.lock().unwrap(); // acquire b while holding a

        // R3: both locks are held simultaneously here — critical work.
        let mut c = count.lock().unwrap();
        match name {
            "t1" => *c |= 1,
            "t2" => *c |= 2,
            _ => {}
        }
    }
    // R7: both locks released here (guards dropped at end of block).

    let _ = name;
}

fn main() {
    // R2: two shared locks; each can be held by only one worker at a time
    // because Mutex guarantees exclusive access.
    let a = Arc::new(Mutex::new(()));
    let b = Arc::new(Mutex::new(()));
    let count = Arc::new(Mutex::new(0u32));

    let a1 = Arc::clone(&a);
    let b1 = Arc::clone(&b);
    let c1 = Arc::clone(&count);
    let h1 = thread::spawn(move || worker("t1", a1, b1, c1)); // R1: worker t1

    let a2 = Arc::clone(&a);
    let b2 = Arc::clone(&b);
    let c2 = Arc::clone(&count);
    let h2 = thread::spawn(move || worker("t2", a2, b2, c2)); // R1: worker t2

    // R6: main waits for both workers to finish.
    h1.join().unwrap();
    h2.join().unwrap();

    let c = *count.lock().unwrap();
    // R9: exactly the required line, then exit.
    if c == 3 {
        println!("DONE t1=1 t2=1");
    } else {
        println!("DONE t1={} t2={}", (c & 1), ((c >> 1) & 1));
    }
}
