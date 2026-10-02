mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::mpsc::{sync_channel, Receiver, SyncSender};
use std::sync::{Arc};
use std::thread;

fn s(lock: Arc<Mutex<i32>>, ch1: SyncSender<i32>, ch2: Receiver<i32>) {
    {
        let mut visits = lock.lock().unwrap();
        *visits += 1;
    }

    cir_trace::record("channel_send", "ch1"); ch1.send(7).unwrap();
    cir_trace::record("channel_recv", "ch2"); let reply = ch2.recv().unwrap();
    let _ = reply;
}

fn r(lock: Arc<Mutex<i32>>, ch1: Receiver<i32>, ch2: SyncSender<i32>) {
    {
        let mut visits = lock.lock().unwrap();
        *visits += 1;
    }

    cir_trace::record("channel_recv", "ch1"); let received = ch1.recv().unwrap();
    cir_trace::record("channel_send", "ch2"); ch2.send(received + 1).unwrap();
}

fn main() { cir_trace::init();
    let lock = Arc::new(Mutex::new_named("lock_mutex0#622", 0));
    let (ch1_tx, ch1_rx) = sync_channel::<i32>(0);
    let (ch2_tx, ch2_rx) = sync_channel::<i32>(0);

    let s_lock = Arc::clone(&lock);
    let s_thread = cir_trace::spawn("s#789", move || s(s_lock, ch1_tx, ch2_rx));

    let r_lock = Arc::clone(&lock);
    let r_thread = cir_trace::spawn("r#895", move || r(r_lock, ch1_rx, ch2_tx));

    s_thread.join().unwrap();
    r_thread.join().unwrap();

    let done = if *lock.lock().unwrap() == 2 { 1 } else { 0 };
    println!("DONE done={done}");
 cir_trace::finish();}
