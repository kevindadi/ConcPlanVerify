use concir_sync::Semaphore;
use std::sync::{Arc, Condvar, Mutex};
use std::thread;

struct MainModule {
    m: Mutex<MainGuardData>,
    cv: Condvar,
    g12: Arc<Semaphore>,
    gN: Arc<Semaphore>,
}

struct MainGuardData {
    waiters: i32,
}

fn w1(main: Arc<MainModule>) {
    let mut guard = main.m.lock().unwrap();
    main.g12.add_permits(1);
    let mut proceed = false;
    while !proceed {
        guard = main.cv.wait(guard).unwrap();
        proceed = true;
    }
    guard.waiters -= 1;
}

fn w2(main: Arc<MainModule>) {
    let mut guard = main.m.lock().unwrap();
    main.g12.add_permits(1);
    let mut proceed = false;
    while !proceed {
        guard = main.cv.wait(guard).unwrap();
        proceed = true;
    }
    guard.waiters -= 1;
}

fn notifier(main: Arc<MainModule>) {
    let _gN = main.gN.acquire();
    let _g12_a = main.g12.acquire();
    let _g12_b = main.g12.acquire();
    let _guard = main.m.lock().unwrap();
    main.cv.notify_all();
}

fn print_done() {}

fn main() {
    let main_mod = Arc::new(MainModule {
        m: Mutex::new(MainGuardData { waiters: 2 }),
        cv: Condvar::new(),
        g12: Semaphore::new(0),
        gN: Semaphore::new(0),
    });

    main_mod.gN.add_permits(1);

    let m1 = Arc::clone(&main_mod);
    let m2 = Arc::clone(&main_mod);
    let m3 = Arc::clone(&main_mod);

    let h1 = thread::spawn(move || w1(m1));
    let h2 = thread::spawn(move || w2(m2));
    let h3 = thread::spawn(move || notifier(m3));

    h1.join().unwrap();
    h2.join().unwrap();
    h3.join().unwrap();

    print_done();
    println!("DONE waiters=0");
}
