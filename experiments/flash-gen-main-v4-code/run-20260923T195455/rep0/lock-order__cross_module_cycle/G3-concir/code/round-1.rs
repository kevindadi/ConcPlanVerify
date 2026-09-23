use std::sync::{Arc, Mutex};
use std::thread;

struct Shared {
    a: Mutex<()>,
    b: Mutex<()>,
}

fn t1(shared: Arc<Shared>) {
    {
        let _ga = shared.a.lock().unwrap();
        let _gb = shared.b.lock().unwrap();
    }
}

fn t2(shared: Arc<Shared>) {
    {
        let _ga = shared.a.lock().unwrap();
        let _gb = shared.b.lock().unwrap();
    }
}

fn main() {
    let shared = Arc::new(Shared {
        a: Mutex::new(()),
        b: Mutex::new(()),
    });

    let s1 = Arc::clone(&shared);
    let h1 = thread::spawn(move || t1(s1));

    let s2 = Arc::clone(&shared);
    let h2 = thread::spawn(move || t2(s2));

    h1.join().unwrap();
    h2.join().unwrap();

    println!("DONE done=1");
}
