mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

struct Resources {
    a: Mutex<()>,
    b: Mutex<()>,
}

fn t1(res: Arc<Resources>) {
    {
        let _a = res.a.lock().unwrap();
        let _b = res.b.lock().unwrap();
    }
}

fn t2(res: Arc<Resources>) {
    {
        let _a = res.a.lock().unwrap();
        let _b = res.b.lock().unwrap();
    }
}

fn main() { cir_trace::init();
    let res = Arc::new(Resources {
        a: Mutex::new_named("res_mutex0", ()),
        b: Mutex::new_named("res_mutex1", ()),
    });

    let res1 = Arc::clone(&res);
    let h1 = cir_trace::spawn("h1", move || t1(res1));

    let res2 = Arc::clone(&res);
    let h2 = cir_trace::spawn("h2", move || t2(res2));

    h1.join().unwrap();
    h2.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
