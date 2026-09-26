mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use concir_sync::Semaphore;
use std::sync::{atomic::{AtomicUsize, Ordering};
use std::thread;

fn main() { cir_trace::init();
    let a = Arc::new(Mutex::new_named("a_mutex0", ()));
    let b = Arc::new(Mutex::new_named("b_mutex0", ()));
    let sa = Semaphore::new_named("sa_semaphore0", 1);
    let sb = Semaphore::new_named("sb_semaphore0", 1);
    let flag = Arc::new(Mutex::new_named("flag_mutex0", (0usize, 0usize)));
    let progress = Arc::new(AtomicUsize::new(0));

    // Reserve each permit so its worker can release it as its first-lock signal.
    let sa_token = sa.acquire();
    let sb_token = sb.acquire();

    let a_lock = Arc::clone(&a);
    let b_lock = Arc::clone(&b);
    let sa_worker = Arc::clone(&sa);
    let sb_worker = Arc::clone(&sb);
    let flag_worker = Arc::clone(&flag);
    let worker_a = thread::Builder::new()
        .name("a".to_owned())
        .spawn(move || {
            sa_token.release();

            // Wait until b has taken its first lock before trying the second.
            let peer_first = sb_worker.acquire();
            peer_first.release();

            loop {
                let first = a_lock.lock().unwrap();
                match b_lock.try_lock() {
                    Ok(second) => {
                        flag_worker.lock().unwrap().0 += 1;
                        drop(second);
                        drop(first);
                        break;
                    }
                    Err(TryLockError::WouldBlock) => {
                        drop(first);
                        thread::yield_now();
                    }
                    Err(TryLockError::Poisoned(error)) => {
                        let second = error.into_inner();
                        flag_worker.lock().unwrap().0 += 1;
                        drop(second);
                        drop(first);
                        break;
                    }
                }
            }
        })
        .unwrap();

    let a_lock = Arc::clone(&a);
    let b_lock = Arc::clone(&b);
    let sa_worker = Arc::clone(&sa);
    let sb_worker = Arc::clone(&sb);
    let flag_worker = Arc::clone(&flag);
    let worker_b = thread::Builder::new()
        .name("b".to_owned())
        .spawn(move || {
            sb_token.release();

            // Wait until a has taken its first lock before trying the second.
            let peer_first = sa_worker.acquire();
            peer_first.release();

            loop {
                let first = b_lock.lock().unwrap();
                match a_lock.try_lock() {
                    Ok(second) => {
                        flag_worker.lock().unwrap().1 += 1;
                        drop(second);
                        drop(first);
                        break;
                    }
                    Err(TryLockError::WouldBlock) => {
                        drop(first);
                        thread::yield_now();
                    }
                    Err(TryLockError::Poisoned(error)) => {
                        let second = error.into_inner();
                        flag_worker.lock().unwrap().1 += 1;
                        drop(second);
                        drop(first);
                        break;
                    }
                }
            }
        })
        .unwrap();

    let bystander_progress = Arc::clone(&progress);
    let _bystander = thread::Builder::new()
        .name("bystander".to_owned())
        .spawn(move || loop {
            bystander_progress.fetch_add(1, Ordering::Relaxed);
            thread::yield_now();
        })
        .unwrap();

    worker_a.join().unwrap();
    worker_b.join().unwrap();

    let counts = flag.lock().unwrap();
    println!("DONE a={} b={}", counts.0, counts.1);
 cir_trace::finish();}
