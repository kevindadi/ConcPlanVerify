mod cir_trace;
use concir_sync::Semaphore;

fn w1(s: &Semaphore) {
    let permit = s.acquire();
    let mut worked = 0;
    worked = 1;
    let _ = worked;
    permit.release();
}

fn w2(s: &Semaphore) {
    let permit = s.acquire();
    let mut worked = 0;
    worked = 1;
    let _ = worked;
    permit.release();
}

fn w3(s: &Semaphore) {
    let permit = s.acquire();
    let mut worked = 0;
    worked = 1;
    let _ = worked;
    permit.release();
}

fn supervisor() {
    let s = Semaphore::new_named("s_semaphore0", 2);
    std::thread::scope(|scope| {
        scope.spawn(|| w1(&s));
        scope.spawn(|| w2(&s));
        scope.spawn(|| w3(&s));
    });
}

fn main() { cir_trace::init();
    let h = cir_trace::spawn("h", supervisor);
    h.join().unwrap();
    println!("DONE done=1");
 cir_trace::finish();}
