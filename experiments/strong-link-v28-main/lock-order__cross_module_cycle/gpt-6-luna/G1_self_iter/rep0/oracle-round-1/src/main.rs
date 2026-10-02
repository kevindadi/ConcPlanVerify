#![allow(non_snake_case, non_upper_case_globals)]

mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};


mod first {
    use crate::cir_trace::sync::{Mutex};

    pub static a: Mutex<()> = Mutex::new_named("a_mutex0", ());

    pub fn t1() {
        let _a = a.lock().unwrap();
        let _b = crate::second::b.lock().unwrap();

        std::hint::black_box((&*_a, &*_b));
    }
}

mod second {
    use crate::cir_trace::sync::{Mutex};

    pub static b: Mutex<()> = Mutex::new_named("b_mutex0", ());

    pub fn t2() {
        let _a = crate::first::a.lock().unwrap();
        let _b = b.lock().unwrap();

        std::hint::black_box((&*_a, &*_b));
    }
}

fn main() { cir_trace::init();
    let t1 = cir_trace::spawn("t1#589", first::t1);
    let t2 = cir_trace::spawn("t2#633", second::t2);

    t1.join().unwrap();
    t2.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
