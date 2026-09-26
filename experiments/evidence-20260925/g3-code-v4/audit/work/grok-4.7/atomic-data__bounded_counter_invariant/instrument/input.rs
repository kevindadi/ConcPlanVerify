use std::sync::Mutex;
use std::thread;

/// Shared counter `c`, declared to range from 0 to 2, protected by `m`.
struct Shared {
    c: i32,
}

fn main() {
    let m = Mutex::new(Shared { c: 0 });
    thread::scope(|scope| {
        scope.spawn(|| w1(&m));
        scope.spawn(|| w2(&m));
    });
    println!("DONE done=1");
}

fn w1(m: &Mutex<Shared>) {
    let mut tmp: i32 = 0;
    {
        let mut guard = m.lock().unwrap();
        tmp = guard.c;
        tmp = tmp + 1;
        guard.c = tmp;
        assert!(guard.c >= 0 && guard.c <= 2);
    }
}

fn w2(m: &Mutex<Shared>) {
    let mut tmp: i32 = 0;
    {
        let mut guard = m.lock().unwrap();
        tmp = guard.c;
        tmp = tmp + 1;
        guard.c = tmp;
        assert!(guard.c >= 0 && guard.c <= 2);
    }
}
