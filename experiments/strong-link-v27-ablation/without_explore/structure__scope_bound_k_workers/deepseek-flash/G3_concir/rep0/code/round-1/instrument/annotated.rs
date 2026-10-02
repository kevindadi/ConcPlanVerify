mod cir_trace;
use concir_sync::Semaphore;
use std::sync::Arc;
use std::thread;

fn w1(s: Arc<Semaphore>) {
    let permit = s.acquire();
    permit.release();
}

fn w2(s: Arc<Semaphore>) {
    let permit = s.acquire();
    permit.release();
}

fn w3(s: Arc<Semaphore>) {
    let permit = s.acquire();
    permit.release();
}

fn main() { cir_trace::init();
    let s = Semaphore::new_named("s_semaphore0#347", 1);

    let s1 = Arc::clone(&s);
    let h1 = cir_trace::spawn("w1#398", move || w1(s1));
    let s2 = Arc::clone(&s);
    let h2 = cir_trace::spawn("w1#471", move || w1(s2));
    let s3 = Arc::clone(&s);
    let h3 = cir_trace::spawn("w2#544", move || w2(s3));
    let s4 = Arc::clone(&s);
    let h4 = cir_trace::spawn("w2#617", move || w2(s4));
    let s5 = Arc::clone(&s);
    let h5 = cir_trace::spawn("w3#690", move || w3(s5));
    let s6 = Arc::clone(&s);
    let h6 = cir_trace::spawn("w3#763", move || w3(s6));

    h1.join().unwrap();
    h2.join().unwrap();
    h3.join().unwrap();
    h4.join().unwrap();
    h5.join().unwrap();
    h6.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
