use concir_sync::Semaphore;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc, Mutex, TryLockError,
};
use std::thread;

fn main() {
    let a = Arc::new(Mutex::new(()));
    let b = Arc::new(Mutex::new(()));
    let sa = Semaphore::new(1);
    let sb = Semaphore::new(1);
    let flag = Arc::new(Mutex::new((0usize, 0usize)));
    let progress = Arc::new(AtomicUsize::new(0));

    // Hold the initial permits so each worker can release one as its signal.
    let sa_token = sa.acquire();
    let sb_token = sb.acquire();

    let a_lock_a = Arc::clone(&a);
    let b_lock_a = Arc::clone(&b);
    let sa_a = Arc::clone(&sa);
    let sb_a = Arc::clone(&sb);
    let flag_a = Arc::clone(&flag);
    let worker_a = thread::Builder::new()
        .name("a".to_owned())
        .spawn(move || {
            let first = a_lock_a.lock().unwrap();
            sa_token.release();

            // b releases sb only after it has taken its first lock.
            let peer_first = sb_a.acquire();
            drop(peer_first);

            loop {
                match b_lock_a.try_lock() {
                    Ok(second) => {
                        flag_a.lock().unwrap().0 += 1;
                        drop(second);
                        drop(first);
                        break;
                    }
                    Err(TryLockError::WouldBlock) => {
                        drop(first);
                        thread::yield_now();

                        // Reacquire the first lock before trying the second again.
                        let first = a_lock_a.lock().unwrap();
                        match b_lock_a.try_lock() {
                            Ok(second)
