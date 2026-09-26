mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{mpsc, Arc};
use std::thread;

use concir_sync::Semaphore;

fn s(lock: Arc<Mutex<i32>>, ch1: mpsc::SyncSender<i32>, ch2: mpsc::Receiver<i32>) {
    let payload = {
        let mut held = lock.lock().unwrap();
        *held += 1;
        1
    };
    cir_trace::record("channel_send", "ch1"); ch1.send(payload).unwrap();
    cir_trace::record("channel_recv", "ch2"); let _reply = ch2.recv().unwrap();
}

fn r(lock: Arc<Mutex<i32>>, ch1: mpsc::Receiver<i32>, ch2: mpsc::SyncSender<i32>) -> i32 {
    {
        let mut held = lock.lock().unwrap();
        *held += 1;
    }
    cir_trace::record("channel_recv", "ch1"); let value = ch1.recv().unwrap();
    cir_trace::record("channel_send", "ch2"); ch2.send(value).unwrap();
    value
}

fn main() { cir_trace::init();
    let _sem = Semaphore::new_named("_sem_semaphore0", 1);
    let lock = Arc::new(Mutex::new_named("lock_mutex0", 0));
    let ch1 = mpsc::sync_channel(0);
    let ch2 = mpsc::sync_channel(0);
    let (ch1_tx, ch1_rx) = ch1;
    let (ch2_tx, ch2_rx) = ch2;

    let lock_s = Arc::clone(&lock);
    let s_thread = cir_trace::spawn("s", move || s(lock_s, ch1_tx, ch2_rx));

    let lock_r = Arc::clone(&lock);
    let r_thread = cir_trace::spawn("r", move || r(lock_r, ch1_rx, ch2_tx));

    s_thread.join().unwrap();
    let done = r_thread.join().unwrap();
    println!("DONE done={done}");
 cir_trace::finish();}
