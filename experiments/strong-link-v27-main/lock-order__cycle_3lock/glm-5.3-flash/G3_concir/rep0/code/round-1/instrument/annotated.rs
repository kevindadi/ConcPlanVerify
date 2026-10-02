mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{OnceLock};

static A: OnceLock<Mutex<()>> = OnceLock::new();
static B: OnceLock<Mutex<()>> = OnceLock::new();
static C: OnceLock<Mutex<()>> = OnceLock::new();

fn a() -> &'static Mutex<()> { A.get_or_init(|| Mutex::new(())) }
fn b() -> &'static Mutex<()> { B.get_or_init(|| Mutex::new(())) }
fn c() -> &'static Mutex<()> { C.get_or_init(|| Mutex::new(())) }

fn t1() {
    let mut work: i64 = 0;
    let _ga = a().lock().unwrap();
    let _gb = b().lock().unwrap();
    work = work + 1;
    drop(_gb);
    drop(_ga);
}

fn t2() {
    let mut work: i64 = 0;
    let _gb = b().lock().unwrap();
    let _gc = c().lock().unwrap();
    work = work + 1;
    drop(_gc);
    drop(_gb);
}

fn t3() {
    let mut work: i64 = 0;
    let _ga = a().lock().unwrap();
    let _gc = c().lock().unwrap();
    work = work + 1;
    drop(_gc);
    drop(_ga);
}

fn main() { cir_trace::init();
    let h1 = cir_trace::spawn("h1#890", t1);
    let h2 = cir_trace::spawn("h2#927", t2);
    let h3 = cir_trace::spawn("h3#964", t3);
    h1.join().unwrap();
    h2.join().unwrap();
    h3.join().unwrap();
    println!("DONE done=1");
 cir_trace::finish();}
