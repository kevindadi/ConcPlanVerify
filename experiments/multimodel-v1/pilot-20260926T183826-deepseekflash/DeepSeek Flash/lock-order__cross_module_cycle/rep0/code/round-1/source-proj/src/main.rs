use std::sync::{Arc, Mutex};
use std::thread;

struct MainModule {
    a: Arc<Mutex<()>>,
}

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

    let a1 = Arc::clone(&main_mod.a);
    let b1 = Arc::clone(&other_mod.b);
    let h1 = thread::spawn(move || {
        t1(a1, b1);
    });

    let a2 = Arc::clone(&main_mod.a);
    let b2 = Arc::clone(&other_mod.b);
    let h2 = thread::spawn(move || {
        t2(a2, b2);
    });

    h1.join().unwrap();
    h2.join().unwrap();

    println!("DONE done=1");
}

fn t1(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>) {
    let _ga = a.lock().unwrap();
    let _gb = b.lock().unwrap();
    drop(_gb);
    drop(_ga);
}

fn t2(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>) {
    let _ga = a.lock().unwrap();
    let _gb = b.lock().unwrap();
    drop(_gb);
    drop(_ga);
}
