use std::sync::{Arc, Mutex};
use std::thread;

struct MainModule {
    a: Mutex<()>,
}

struct OtherModule {
    b: Mutex<()>,
}

fn t1(main_mod: &MainModule, other_mod: &OtherModule) {
    {
        let _a = main_mod.a.lock().unwrap();
        {
            let _b = other_mod.b.lock().unwrap();
        }
    }
}

fn t2(main_mod: &MainModule, other_mod: &OtherModule) {
    {
        let _a = main_mod.a.lock().unwrap();
        {
            let _b = other_mod.b.lock().unwrap();
        }
    }
}

fn main() {
    let main_mod = Arc::new(MainModule { a: Mutex::new(()) });
    let other_mod = Arc::new(OtherModule { b: Mutex::new(()) });

    let main_mod_t1 = Arc::clone(&main_mod);
    let other_mod_t1 = Arc::clone(&other_mod);
    let h1 = thread::spawn(move || {
        t1(&main_mod_t1, &other_mod_t1);
    });

    let main_mod_t2 = Arc::clone(&main_mod);
    let other_mod_t2 = Arc::clone(&other_mod);
    let h2 = thread::spawn(move || {
        t2(&main_mod_t2, &other_mod_t2);
    });

    h1.join().unwrap();
    h2.join().unwrap();

    println!("DONE done=1");
}
