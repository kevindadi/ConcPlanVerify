mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{mpsc::{sync_channel, Receiver, SyncSender};
use std::thread;

fn sender(ch: SyncSender<u32>, m: Arc<Mutex<usize>>) {
    for value in [10, 20] {
        {
            let mut count = m.lock().unwrap();
            *count += 1;
        } // Release m before a send can wait.
        cir_trace::record("channel_send", "ch"); ch.send(value).unwrap();
    }
}

fn receiver(ch: Receiver<u32>, m: Arc<Mutex<usize>>) {
    for expected in [10, 20] {
        cir_trace::record("channel_recv", "ch"); let value = ch.recv().unwrap();
        assert_eq!(value, expected);

        {
            let mut count = m.lock().unwrap();
            *count += 1;
        } // Release m before the next receive can wait.
    }
}

fn main() { cir_trace::init();
    let ch = sync_channel::<u32>(1);
    let (ch_tx, ch_rx) = ch;
    let m = Arc::new(Mutex::new_named("m_mutex0#774", 0usize));

    let sender_m = Arc::clone(&m);
    let receiver_m = Arc::clone(&m);

    let sender_thread = cir_trace::spawn("sender#886", move || sender(ch_tx, sender_m));
    let receiver_thread = cir_trace::spawn("receiver#960", move || receiver(ch_rx, receiver_m));

    sender_thread.join().unwrap();
    receiver_thread.join().unwrap();

    let done = 1;
    println!("DONE done={done}");
 cir_trace::finish();}
