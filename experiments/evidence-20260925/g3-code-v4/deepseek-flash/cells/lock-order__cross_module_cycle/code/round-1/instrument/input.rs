use std::sync::{Arc, Mutex};
use std::thread;

struct MainModule {
    a: Arc<Mutex<()>>,
}

struct OtherModule {
    b: Arc<Mutex<()>>,
}

fn main() {
    let main_mod = Arc::new(MainModule {
        a: Arc::new(Mutex::new(())),
    });
    let other_mod = Arc::new(OtherModule {
        b: Arc::new(Mutex::new(())),
    });

    let m1 = Arc::clone(&main_mod);
    let o1 = Arc::clone(&other_mod);
    let h1 = thread::spawn(move || {
        t1(&m1, &o1);
    });

    let m2 = Arc::clone(&main_mod);
    let o2 = Arc::clone(&other_mod);
    let h2 = thread::spawn(move || {
        t2(&m2, &o2);
    });

    h1.join().unwrap();
    h2.join().unwrap();

    println!("DONE done=1");
}

fn t1(main_mod: &MainModule, other_mod: &OtherModule) {
    let _ga = main_mod.a.lock().unwrap();
    let _gb = other_mod.b.lock().unwrap();
    drop(_gb);
    drop(_ga);
}

fn t2(main_mod: &MainModule, other_mod: &OtherModule) {
    let _ga = main_mod.a.lock().unwrap();
    let _gb = other_mod.b.lock().unwrap();
    drop(_gb);
    drop(_ga);
}
