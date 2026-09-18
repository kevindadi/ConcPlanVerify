//! Reference Rust for tests/e2e/channel_deadlock/fixed.json.
//! Fix: the receiver performs recv() before taking the mutex, so it never
//! blocks on the channel while holding the lock the sender needs.

use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::thread;

fn main() {
    let mtx = Arc::new(Mutex::new(()));
    let (tx, rx) = mpsc::channel::<i32>();

    let mtx_s = Arc::clone(&mtx);
    let sender = thread::spawn(move || {
        let guard = mtx_s.lock().unwrap(); // s1: lock mtx
        tx.send(42).unwrap(); // s2: send 42
        drop(guard); // s3: drop mtx
    });

    let mtx_r = Arc::clone(&mtx);
    let receiver = thread::spawn(move || {
        let _value = rx.recv().unwrap(); // s1: recv first, without holding mtx
        let guard = mtx_r.lock().unwrap(); // s2: lock mtx
        drop(guard); // s3: drop mtx
    });

    sender.join().unwrap();
    receiver.join().unwrap();
}
