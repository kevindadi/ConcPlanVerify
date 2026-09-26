mod cir_trace;
use concir_sync::Semaphore;
use std::thread;

fn w1(s: &Semaphore) {
    let permit = s.acquire();
    drop(permit);
}

fn w2(s: &Semaphore) {
    let permit = s.acquire();
    drop(permit);
}

fn w3(s: &Semaphore) {
    let permit = s.acquire();
    drop(permit);
}

fn main() { cir_trace::init();
    let s = Semaphore::new_named("s_semaphore0", 1);

    thread::scope(|scope| {
        let s = &*s;
        // At most two activations of each role run at once.
        for _ in 0..2 {
            scope.spawn(|| w1(s));
            scope.spawn(|| w2(s));
            scope.spawn(|| w3(s));
        }
    });

    let done = 1;
    println!("DONE done={done}");
 cir_trace::finish();}
