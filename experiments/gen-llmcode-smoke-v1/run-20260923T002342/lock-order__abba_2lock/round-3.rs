use std::sync::{Arc, Mutex};
use std::thread;

struct Shared {
    a: Mutex<()>,
    b: Mutex<()>,
    done1: Mutex<bool>,
    done2: Mutex<bool>,
}

fn t1(shared: Arc<Shared>) {
    {
        let _ga = shared.a.lock().unwrap();
        let _gb = shared.b.lock().unwrap();
        {
            let mut d = shared.done1.lock().unwrap();
            *d = true;
        }
    }
}

fn t2(shared: Arc<Shared>) {
    {
        let _ga = shared.a.lock().unwrap();
        let _gb = shared.b.lock().unwrap();
        {
            let mut d = shared.done2.lock().unwrap();
            *d = true;
        }
    }
}

fn main() {
    let shared = Arc::new(Shared {
        a: Mutex::new(()),
        b: Mutex::new(()),
        done1: Mutex::new(false),
        done2: Mutex::new(false),
    });

    let s1 = Arc::clone(&shared);
    let h1 = thread::spawn(move || t1(s1));

    let s2 = Arc::clone(&shared);
    let h2 = thread::spawn(move || t2(s2));

    h1.join().unwrap();
    h2.join().unwrap();

    let d1 = *shared.done1.lock().unwrap();
    let d2 = *shared.done2.lock().unwrap();

    println!("DONE t1={} t2={}", d1 as u8, d2 as u8);
}
