use concir_sync::Semaphore;

fn w1(s: &Semaphore) {
    let permit = s.acquire();
    permit.release();
}

fn w2(s: &Semaphore) {
    let permit = s.acquire();
    permit.release();
}

fn main() {
    let s = Semaphore::new(1);

    let h1 = {
        let s = s.clone();
        std::thread::spawn(move || w1(&s))
    };
    let h2 = {
        let s = s.clone();
        std::thread::spawn(move || w2(&s))
    };

    h1.join().unwrap();
    h2.join().unwrap();

    println!("DONE permits=1");
}
