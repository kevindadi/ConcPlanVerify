use std::sync::{Arc, Mutex};
use std::thread;

fn main() {
    let m = Arc::new(Mutex::new(0i32));

    let w1 = {
        let m = Arc::clone(&m);
        move || {
            let mut c = m.lock().unwrap();
            if *c < 1 {
                *c = *c + 1;
            }
            drop(c);
        }
    };

    let w2 = {
        let m = Arc::clone(&m);
        move || {
            let mut c = m.lock().unwrap();
            if *c < 1 {
                *c = *c + 1;
            }
            drop(c);
        }
    };

    let h1 = thread::spawn(w1);
    let h2 = thread::spawn(w2);

    h1.join().unwrap();
    h2.join().unwrap();

    println!("DONE done=1");
}
