mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

fn t1(a: &Arc<Mutex<()>>, b: &Arc<Mutex<()>>) {
    let _g1 = a.lock().unwrap();
    let _g2 = b.lock().unwrap();
    drop(_g2);
    drop(_g1);
}

fn t2(a: &Arc<Mutex<()>>, b: &Arc<Mutex<()>>) {
    let _g1 = a.lock().unwrap();
    let _g2 = b.lock().unwrap();
    drop(_g2);
    drop(_g1);
}

fn t3(c: &Arc<Mutex<()>>, d: &Arc<Mutex<()>>) {
    let _g1 = c.lock().unwrap();
    let _g2 = d.lock().unwrap();
    drop(_g2);
    drop(_g1);
}

fn t4(c: &Arc<Mutex<()>>, d: &Arc<Mutex<()>>) {
    let _g1 = c.lock().unwrap();
    let _g2 = d.lock().unwrap();
    drop(_g2);
    drop(_g1);
}

fn main() { cir_trace::init();
    let a = Arc::new(Mutex::new_named("a_mutex0#675", ()));
    let b = Arc::new(Mutex::new_named("b_mutex0#713", ()));
    let c = Arc::new(Mutex::new_named("c_mutex0#751", ()));
    let d = Arc::new(Mutex::new_named("d_mutex0#789", ()));

    let h1 = {
        let a = Arc::clone(&a);
        let b = Arc::clone(&b);
        cir_trace::spawn("t1#887", move || t1(&a, &b))
    };
    let h2 = {
        let a = Arc::clone(&a);
        let b = Arc::clone(&b);
        cir_trace::spawn("t2#1015", move || t2(&a, &b))
    };
    let h3 = {
        let c = Arc::clone(&c);
        let d = Arc::clone(&d);
        cir_trace::spawn("t3#1143", move || t3(&c, &d))
    };
    let h4 = {
        let c = Arc::clone(&c);
        let d = Arc::clone(&d);
        cir_trace::spawn("t4#1271", move || t4(&c, &d))
    };

    h1.join().unwrap();
    h2.join().unwrap();
    h3.join().unwrap();
    h4.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
