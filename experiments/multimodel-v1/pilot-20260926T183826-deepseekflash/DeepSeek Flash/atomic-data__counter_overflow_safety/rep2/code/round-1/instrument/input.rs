use std::sync::{Arc, Mutex};
use std::thread;

struct Shared {
    m: Mutex<i32>,
}

fn w1(shared: Arc<Shared>) {
    {
        let mut guard = shared.m.lock().unwrap();
        if *guard < 1 {
            *guard = 1;
        }
    }
}

fn w2(shared: Arc<Shared>) {
    {
        let mut guard = shared.m.lock().unwrap();
        if *guard < 1 {
            *guard = 1;
        }
    }
}

fn main() {
    let shared = Arc::new(Shared { m: Mutex::new(0) });

    let s1 = Arc::clone(&shared);
    let s2 = Arc::clone(&shared);

    let h1 = thread::spawn(move || w1(s1));
    let h2 = thread::spawn(move || w2(s2));

    h1.join().unwrap();
    h2.join().unwrap();

    let done = *shared.m.lock().unwrap();
    println!("DONE done={}", done);
}
