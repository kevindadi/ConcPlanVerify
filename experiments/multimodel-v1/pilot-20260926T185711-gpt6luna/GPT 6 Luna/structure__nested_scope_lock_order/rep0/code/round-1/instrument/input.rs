use std::sync::Mutex;
use std::thread;

fn x1(a: &Mutex<()>, b: &Mutex<()>) {
    let guard_a = a.lock().unwrap();
    let guard_b = b.lock().unwrap();
    drop(guard_b);
    drop(guard_a);
}

fn x2(a: &Mutex<()>, b: &Mutex<()>) {
    let guard_a = a.lock().unwrap();
    let guard_b = b.lock().unwrap();
    drop(guard_b);
    drop(guard_a);
}

fn outer(a: &Mutex<()>, b: &Mutex<()>) {
    thread::scope(|scope| {
        scope.spawn(|| x1(a, b));
        scope.spawn(|| x2(a, b));
    });
}

fn main() {
    let a = Mutex::new(());
    let b = Mutex::new(());

    thread::scope(|scope| {
        scope.spawn(|| outer(&a, &b));
    });

    println!("DONE done=1");
}
