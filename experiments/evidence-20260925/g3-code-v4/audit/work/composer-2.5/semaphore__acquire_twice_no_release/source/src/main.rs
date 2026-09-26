use concir_sync::Semaphore;

fn w1(s: Semaphore) {
    s.acquire();
    let mut work = 0;
    work = 1;
    s.release();

    s.acquire();
    work = 2;
    s.release();
}

fn w2(s: Semaphore) {
    s.acquire();
    let mut work = 0;
    work = 1;
    s.release();

    s.acquire();
    work = 2;
    s.release();
}

fn main() {
    let s = Semaphore::new(1);

    let s_w1 = s.clone();
    let j1 = cir_trace::spawn("main::w1", move || w1(s_w1));

    let s_w2 = s.clone();
    let j2 = cir_trace::spawn("main::w2", move || w2(s_w2));

    j1.join().unwrap();
    j2.join().unwrap();

    println!("DONE done=1");
}
