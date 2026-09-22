use std::sync::{Arc, Mutex};
use std::thread;

fn main() {
    let m = Arc::new(Mutex::new(0u32));
    let c = Arc::new(Mutex::new(0u32));

    let mut handles = Vec::new();

    {
        let m = Arc::clone(&m);
        let c = Arc::clone(&c);
        handles.push(thread::spawn(move || {
            let _guard = m.lock().unwrap();
            let mut val = c.lock().unwrap();
            *val += 1;
        }));
    }

    {
        let m = Arc::clone(&m);
        let c = Arc::clone(&c);
        handles.push(thread::spawn(move || {
            let _guard = m.lock().unwrap();
            let mut val = c.lock().unwrap();
            *val += 1;
        }));
    }

    for h in handles {
        h.join().unwrap();
    }

    let done = *c.lock().unwrap();
    println!("DONE done={}", done);
}
