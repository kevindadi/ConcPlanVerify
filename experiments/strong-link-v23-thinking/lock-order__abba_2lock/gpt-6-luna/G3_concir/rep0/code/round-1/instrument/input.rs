use std::sync::Mutex;
use std::thread;

fn t1(a: &Mutex<()>, b: &Mutex<()>) -> i32 {
    let _a = a.lock().unwrap();
    let _b = b.lock().unwrap();
    let work = 1;
    drop(_b);
    drop(_a);
    work
}

fn t2(a: &Mutex<()>, b: &Mutex<()>) -> i32 {
    let _a = a.lock().unwrap();
    let _b = b.lock().unwrap();
    let work = 1;
    drop(_b);
    drop(_a);
    work
}

fn main() {
    let a = Mutex::new(());
    let b = Mutex::new(());

    let (result1, result2) = thread::scope(|scope| {
        let handle1 = scope.spawn(|| t1(&a, &b));
        let handle2 = scope.spawn(|| t2(&a, &b));

        (handle1.join().unwrap(), handle2.join().unwrap())
    });

    println!("DONE t1={} t2={}", result1, result2);
}
