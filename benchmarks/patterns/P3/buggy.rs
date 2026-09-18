//! Reference Rust for tests/e2e/channel_deadlock/buggy.json.
//! Expected defect: ChannelBlock. The receiver blocks on recv() while holding
//! the mutex; the sender needs that mutex before it can send, so neither
//! thread can make progress.
//! Running this program can hang forever; it exists for Lockbud/Miri baselines.

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
        let guard = mtx_r.lock().unwrap(); // s1: lock mtx (holds it across recv!)
        let _value = rx.recv().unwrap(); // s2: recv blocks while sender waits on mtx
        drop(guard); // s3: drop mtx
    });

    sender.join().unwrap();
    receiver.join().unwrap();
}
