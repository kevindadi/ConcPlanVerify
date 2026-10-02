use concir_sync::Semaphore;

fn main() {
    let s = Semaphore::new(2);

    let s1 = s.clone();
    let h1 = std::thread::spawn(move || {
        w1(&s1);
    });

    let s2 = s.clone();
    let h2 = std::thread::spawn(move || {
        w2(&s2);
    });

    h1.join().expect("w1 panicked");
    h2.join().expect("w2 panicked");

    println!("DONE done=1");
}

fn w1(s: &Semaphore) {
    let mut work: i32 = 0;

    let permit = s.acquire();
    work = work + 1;
    permit.release();

    let permit = s.acquire();
    work = work + 1;
    permit.release();
}

fn w2(s: &Semaphore) {
    let mut work: i32 = 0;

    let permit = s.acquire();
    work = work + 1;
    permit.release();

    let permit = s.acquire();
    work = work + 1;
    permit.release();
}
