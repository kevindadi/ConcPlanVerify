mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
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

fn main() { cir_trace::init();
    let main_mod = Arc::new(MainModule { a: Mutex::new_named("main_mod_mutex0", ()) });
    let other_mod = Arc::new(OtherModule { b: Mutex::new_named("other_mod_mutex0", ()) });

    let main_mod_t1 = Arc::clone(&main_mod);
    let other_mod_t1 = Arc::clone(&other_mod);
    let h1 = cir_trace::spawn("h1", move || {
        t1(&main_mod_t1, &other_mod_t1);
    });

    let main_mod_t2 = Arc::clone(&main_mod);
    let other_mod_t2 = Arc::clone(&other_mod);
    let h2 = cir_trace::spawn("h2", move || {
        t2(&main_mod_t2, &other_mod_t2);
    });

    h1.join().unwrap();
    h2.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
