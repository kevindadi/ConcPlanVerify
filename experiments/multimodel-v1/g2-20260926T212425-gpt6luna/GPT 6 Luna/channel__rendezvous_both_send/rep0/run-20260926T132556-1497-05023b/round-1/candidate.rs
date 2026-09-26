use concir_sync::Semaphore;
use std::sync::mpsc::{sync_channel, Receiver, SyncSender};
use std::sync::{Arc, Mutex};

fn s1(tx: SyncSender<i32>) {
    tx.send(1).unwrap();
}

fn r(rx: Receiver<i32>, done: Arc<Mutex<usize>>, lock: Arc<Semaphore>) {
    let _value = rx.recv().unwrap();

    let _permit = lock.acquire();
    *done.lock().unwrap() = 1;
}

fn main() {
    let ch = sync_channel::<i32>(0);
    let (tx, rx) = ch;

    let done = Arc::new(Mutex::new(0));
    let lock = Semaphore::new(1);

    let s1_task = std::thread::spawn(move || s1(tx));
    let r_task = {
        let done = Arc::clone(&done);
        let lock = Arc::clone(&lock);
        std::thread::spawn(move || r(rx, done, lock))
    };

    s1_task.join().unwrap();
    r_task.join().unwrap();

    let _permit = lock.acquire();
    let done = *done.lock().unwrap();
    println!("DONE done={}", done);
}
