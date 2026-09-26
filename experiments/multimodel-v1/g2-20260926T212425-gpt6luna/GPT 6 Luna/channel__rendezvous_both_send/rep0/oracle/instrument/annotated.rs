mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use concir_sync::Semaphore;
use std::sync::mpsc::{sync_channel, Receiver, SyncSender};
use std::sync::{Arc};

fn s1(tx: SyncSender<i32>) {
    cir_trace::record("channel_send", "tx"); tx.send(1).unwrap();
}

fn r(rx: Receiver<i32>, done: Arc<Mutex<usize>>, lock: Arc<Semaphore>) {
    cir_trace::record("channel_recv", "rx"); let _value = rx.recv().unwrap();

    let _permit = lock.acquire();
    *done.lock().unwrap() = 1;
}

fn main() { cir_trace::init();
    let ch = sync_channel::<i32>(0);
    let (tx, rx) = ch;

    let done = Arc::new(Mutex::new_named("done_mutex0", 0));
    let lock = Semaphore::new_named("lock_semaphore0", 1);

    let s1_task = cir_trace::spawn("s1", move || s1(tx));
    let r_task = {
        let done = Arc::clone(&done);
        let lock = Arc::clone(&lock);
        cir_trace::spawn("r", move || r(rx, done, lock))
    };

    s1_task.join().unwrap();
    r_task.join().unwrap();

    let _permit = lock.acquire();
    let done = *done.lock().unwrap();
    println!("DONE done={}", done);
 cir_trace::finish();}
