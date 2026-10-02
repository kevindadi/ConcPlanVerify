#![allow(non_upper_case_globals)]

mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};


static a: Mutex<()> = Mutex::new_named("a_mutex0", ());
static b: Mutex<()> = Mutex::new_named("b_mutex0", ());
static c: Mutex<()> = Mutex::new_named("c_mutex0", ());

fn t1() {
    let mut work: i64 = 0;
    let _ga = a.lock().unwrap();
    let _gb = b.lock().unwrap();
    work = work + 1;
    drop(_gb);
    drop(_ga);
}

fn t2() {
    let mut work: i64 = 0;
    let _gb = b.lock().unwrap();
    let _gc = c.lock().unwrap();
    work = work + 1;
    drop(_gc);
    drop(_gb);
}

fn t3() {
    let mut work: i64 = 0;
    let _ga = a.lock().unwrap();
    let _gc = c.lock().unwrap();
    work = work + 1;
    drop(_gc);
    drop(_ga);
}

fn main() { cir_trace::init();
    let h1 = cir_trace::spawn("t1#669", || t1());
    let h2 = cir_trace::spawn("t2#711", || t2());
    let h3 = cir_trace::spawn("t3#753", || t3());
    h1.join().unwrap();
    h2.join().unwrap();
    h3.join().unwrap();
    println!("DONE done=1");
 cir_trace::finish();}
