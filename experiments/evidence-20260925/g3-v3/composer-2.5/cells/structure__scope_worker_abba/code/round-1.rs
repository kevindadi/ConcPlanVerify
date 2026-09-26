use std::sync::Mutex;

fn w1(a: &Mutex<()>, b: &Mutex<()>) {
    let _ga = a.lock().unwrap();
    let _gb = b.lock().unwrap();
}

fn w2(a: &Mutex<()>, b: &Mutex<()>) {
    let _ga = a.lock().unwrap();
    let _gb = b.lock().unwrap();
}

fn main() {
    let a = Mutex::new(());
    let b = Mutex::new(());

    std::thread::scope(|s| {
        s.spawn(|| w1(&a, &b));
        s.spawn(|| w2(&a, &b));
    });

    println!("DONE done=1");
}
