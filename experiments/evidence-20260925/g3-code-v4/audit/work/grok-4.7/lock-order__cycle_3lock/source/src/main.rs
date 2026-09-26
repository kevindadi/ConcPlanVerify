use std::sync::Mutex;
use std::thread;

fn main() {
    let a = Mutex::new(());
    let b = Mutex::new(());
    let c = Mutex::new(());

    thread::scope(|s| {
        s.spawn(|| t1(&a, &b));
        s.spawn(|| t2(&b, &c));
        s.spawn(|| t3(&a, &c));
    });

    println!("DONE done=1");
}

fn t1(a: &Mutex<()>, b: &Mutex<()>) {
    let guard_a = a.lock().unwrap();
    let guard_b = b.lock().unwrap();
    drop(guard_b);
    drop(guard_a);
}

fn t2(b: &Mutex<()>, c: &Mutex<()>) {
    let guard_b = b.lock().unwrap();
    let guard_c = c.lock().unwrap();
    drop(guard_c);
    drop(guard_b);
}

fn t3(a: &Mutex<()>, c: &Mutex<()>) {
    let guard_a = a.lock().unwrap();
    let guard_c = c.lock().unwrap();
    drop(guard_c);
    drop(guard_a);
}
