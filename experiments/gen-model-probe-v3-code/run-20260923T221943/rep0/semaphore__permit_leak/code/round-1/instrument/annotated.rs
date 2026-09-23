mod cir_trace;
use concir_sync::Semaphore;

fn w1(s: &Semaphore) {
    let permit = s.acquire();
    permit.release();
}

fn w2(s: &Semaphore) {
    let permit = s.acquire();
    permit.release();
}

fn main() { cir_trace::init();
    let s = Semaphore::new_named("s_semaphore0", 1);

    let h1 = {
        let s = s.clone();
        cir_trace::spawn("w1", move || w1(&s))
    };
    let h2 = {
        let s = s.clone();
        cir_trace::spawn("w2", move || w2(&s))
    };

    h1.join().unwrap();
    h2.join().unwrap();

    println!("DONE permits=1");
 cir_trace::finish();}
