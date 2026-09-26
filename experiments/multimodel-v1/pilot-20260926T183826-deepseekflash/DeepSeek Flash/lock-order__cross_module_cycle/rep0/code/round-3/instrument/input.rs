use std::sync::{Arc, Mutex};
use std::thread;

// Module `main` owns resource `a`.
struct MainModule {
    a: Arc<Mutex<()>>,
}

// Module `other` owns resource `b`.
struct OtherModule {
    b: Arc<Mutex<()>>,
}

fn main() {
    let main_mod = MainModule {
        a: Arc::new(Mutex::new(())),
    };
    let other_mod = OtherModule {
        b: Arc::new(Mutex::new(())),
    };

    // main::t1
    let a1 = Arc::clone(&main_mod.a);
    let b1 = Arc::clone(&other_mod.b);
    let h1 = thread::spawn(move || {
        t1(a1, b1);
    });

    // other::t2
    let a2 = Arc::clone(&main_mod.a);
    let b2 = Arc::clone(&other_mod.b);
    let h2 = thread::spawn(move || {
        t2(a2, b2);
    });

    h1.join().unwrap();
    h2.join().unwrap();

    println!("DONE done=1");
}

// main::t1: lock main::a, lock other::b, unlock other::b, unlock main::a
fn t1(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>) {
    let ga = a.lock().unwrap();
    let gb = b.lock().unwrap();
    drop(gb);
    drop(ga);
}

// other::t2: lock main::a, lock other::b, unlock other::b, unlock main::a
fn t2(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>) {
    let ga = a.lock().unwrap();
    let gb = b.lock().unwrap();
    drop(gb);
    drop(ga);
}
