mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::mpsc::{sync_channel, Receiver, SyncSender};
use std::sync::{Arc};
use std::thread;

fn s(ch1: SyncSender<i32>, ch2: Receiver<i32>, shared_lock: Arc<Mutex<usize>>) {
    {
        let mut value = shared_lock.lock().unwrap();
        *value += 1;
    }

    cir_trace::record("channel_send", "ch1"); ch1.send(1).unwrap();
    cir_trace::record("channel_recv", "ch2"); let _reply = ch2.recv().unwrap();
}

fn r(ch1: Receiver<i32>, ch2: SyncSender<i32>, shared_lock: Arc<Mutex<usize>>) {
    {
        let mut value = shared_lock.lock().unwrap();
        *value += 1;
    }

    cir_trace::record("channel_recv", "ch1"); let value = ch1.recv().unwrap();
    cir_trace::record("channel_send", "ch2"); ch2.send(value).unwrap();
}

fn main() { cir_trace::init();
    let (ch1_tx, ch1_rx) = sync_channel::<i32>(0);
    let (ch2_tx, ch2_rx) = sync_channel::<i32>(0);
    let shared_lock = Arc::new(Mutex::new_named("shared_lock_mutex0#731", 0));

    let sender_lock = Arc::clone(&shared_lock);
    let sender = cir_trace::spawn("s#806", move || s(ch1_tx, ch2_rx, sender_lock));

    let receiver_lock = Arc::clone(&shared_lock);
    let receiver = cir_trace::spawn("r#931", move || r(ch1_rx, ch2_tx, receiver_lock));

    sender.join().unwrap();
    receiver.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
