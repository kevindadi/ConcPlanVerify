use std::sync::Mutex;
use std::thread;

fn t1(a: &Mutex<()>, b: &Mutex<()>) -> i32 {
    let guard_a = a.lock().unwrap();
    let guard_b = b.lock().unwrap();
    let work = 1;
    drop(guard_b);
    drop(guard_a);
    work
}

fn t2(a: &Mutex<()>, b: &Mutex<()>) -> i32 {
    let guard_a = a.lock().unwrap();
    let guard_b = b.lock().unwrap();
    let work = 1;
    drop(guard_b);
    drop(guard_a);
    work
}

fn main() {
    let a = Mutex::new(());
    let b = Mutex::new(());

    let (v1, v2) = thread::scope(|scope| {
        let h1 = scope.spawn(|| t1(&a, &b));
        let h2 = scope.spawn(|| t2(&a, &b));
        (h1.join().unwrap(), h2.join().unwrap())
    });

    println!("DONE t1={v1} t2={v2}");
}
