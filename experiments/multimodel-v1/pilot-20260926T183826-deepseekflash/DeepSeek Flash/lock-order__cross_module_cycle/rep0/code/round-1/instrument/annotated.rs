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
    let main_mod = MainModule {
        a: Arc::new(Mutex::new_named("main_mod_mutex0", ())),
    };
    let other_mod = OtherModule {
        b: Arc::new(Mutex::new_named("other_mod_mutex0", ())),
    };

    let a1 = Arc::clone(&main_mod.a);
    let b1 = Arc::clone(&other_mod.b);
    let h1 = cir_trace::spawn("t1", move || {
        t1(a1, b1);
    });

    let a2 = Arc::clone(&main_mod.a);
    let b2 = Arc::clone(&other_mod.b);
    let h2 = cir_trace::spawn("t2", move || {
        t2(a2, b2);
    });

    h1.join().unwrap();
    h2.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}

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
