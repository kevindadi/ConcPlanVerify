mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

struct MainModule {
    a: Arc<Mutex<()>>,
}

struct OtherModule {
    b: Arc<Mutex<()>>,
}

fn main() { cir_trace::init();
    let main_mod = Arc::new(MainModule {
        a: Arc::new(Mutex::new_named("main_mod_mutex0", ())),
    });
    let other_mod = Arc::new(OtherModule {
        b: Arc::new(Mutex::new_named("other_mod_mutex0", ())),
    });

    let m1 = Arc::clone(&main_mod);
    let o1 = Arc::clone(&other_mod);
    let h1 = cir_trace::spawn("t1", move || {
        t1(&m1, &o1);
    });

    let m2 = Arc::clone(&main_mod);
    let o2 = Arc::clone(&other_mod);
    let h2 = cir_trace::spawn("t2", move || {
        t2(&m2, &o2);
    });

    h1.join().unwrap();
    h2.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}

fn t1(main_mod: &MainModule, other_mod: &OtherModule) {
    let ga = main_mod.a.lock().unwrap();
    let gb = other_mod.b.lock().unwrap();
    drop(gb);
    drop(ga);
}

fn t2(main_mod: &MainModule, other_mod: &OtherModule) {
    let ga = main_mod.a.lock().unwrap();
    let gb = other_mod.b.lock().unwrap();
    drop(gb);
    drop(ga);
}
