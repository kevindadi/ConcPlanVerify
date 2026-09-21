use std::sync::{Arc, Mutex};
use std::thread;

fn main() {
    let a = Arc::new(Mutex::new(0));
    let b = Arc::new(Mutex::new(0));
    let c = Arc::new(Mutex::new(0));
    let d = Arc::new(Mutex::new(0));

    let mut handles = Vec::new();

    {
        let a = Arc::clone(&a);
        let b = Arc::clone(&b);
        handles.push(thread::spawn(move || {
            let _ga = a.lock().unwrap();
            let _gb = b.lock().unwrap();
        }));
    }
    {
        let a = Arc::clone(&a);
        let b = Arc::clone(&b);
        handles.push(thread::spawn(move || {
            let _ga = a.lock().unwrap();
            let _gb = b.lock().unwrap();
        }));
    }
    {
        let c = Arc::clone(&c);
        let d = Arc::clone(&d);
        handles.push(thread::spawn(move || {
            let _gc = c.lock().unwrap();
            let _gd = d.lock().unwrap();
        }));
    }
    {
        let c = Arc::clone(&c);
        let d = Arc::clone(&d);
        handles.push(thread::spawn(move || {
            let _gc = c.lock().unwrap();
            let _gd = d.lock().unwrap();
        }));
    }

    for h in handles {
        h.join().unwrap();
    }

    println!("DONE done=1");
}
