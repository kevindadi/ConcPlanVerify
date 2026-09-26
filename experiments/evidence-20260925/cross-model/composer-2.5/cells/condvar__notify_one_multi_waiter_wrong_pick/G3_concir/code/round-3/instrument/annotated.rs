mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use concir_sync::Semaphore;
use std::sync::{Arc};
use std::thread;

struct Main {
    m: Mutex<MainState>,
    cv: Condvar,
    g12: Arc<Semaphore>,
    gN: Arc<Semaphore>,
}

struct MainState {
    waiters: i32,
    proceed: bool,
}

fn w1(main: Arc<Main>) {
    let mut guard = main.m.lock().unwrap();
    main.g12.release(1);
    while !guard.proceed {
        guard = main.cv.wait(guard).unwrap();
    }
    guard.waiters -= 1;
}

fn w2(main: Arc<Main>) {
    let mut guard = main.m.lock().unwrap();
    main.g12.release(1);
    while !guard.proceed {
        guard = main.cv.wait(guard).unwrap();
    }
    guard.waiters -= 1;
}

fn notifier(main: Arc<Main>) {
    let _gN = main.gN.acquire();
    let _g12_1 = main.g12.acquire();
    let _g12_2 = main.g12.acquire();
    let mut guard = main.m.lock().unwrap();
    guard.proceed = true;
    main.cv.notify_all();
}

fn print_done(main: &Main) {
    let guard = main.m.lock().unwrap();
    println!("DONE waiters={}", guard.waiters);
}

fn main() { cir_trace::init();
    let main = Arc::new(Main {
        m: Mutex::new_named("main_mutex0", MainState {
            waiters: 2,
            proceed: false,
        }),
        cv: Condvar::new_named("main_condvar0"),
        g12: Semaphore::new_named("main_semaphore0", 0),
        gN: Semaphore::new_named("main_semaphore1", 0),
    });

    main.gN.release(1);

    let m1 = Arc::clone(&main);
    let m2 = Arc::clone(&main);
    let m3 = Arc::clone(&main);

    let j1 = cir_trace::spawn("w1", move || w1(m1));
    let j2 = cir_trace::spawn("w2", move || w2(m2));
    let j3 = cir_trace::spawn("notifier", move || notifier(m3));

    j1.join().unwrap();
    j2.join().unwrap();
    j3.join().unwrap();

    print_done(&main);
 cir_trace::finish();}
