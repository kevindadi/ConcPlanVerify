mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{mpsc, Arc};
use std::thread;

fn sender(ch: mpsc::SyncSender<i32>, m: Arc<Mutex<usize>>) {
    for value in [1, 2] {
        {
            let mut guard = m.lock().unwrap();
            *guard += 1;
        } // Release m before sending, which may wait.
        cir_trace::record("channel_send", "ch"); ch.send(value).unwrap();
    }
}

fn receiver(ch: mpsc::Receiver<i32>, m: Arc<Mutex<usize>>) {
    for _ in 0..2 {
        {
            let mut guard = m.lock().unwrap();
            *guard += 1;
        } // Release m before receiving, which may wait.
        cir_trace::record("channel_recv", "ch"); ch.recv().unwrap();
    }
}

fn main() { cir_trace::init();
    let (ch_sender, ch_receiver) = mpsc::sync_channel::<i32>(1);
    let ch = (ch_sender, ch_receiver);
    let m = Arc::new(Mutex::new_named("m_mutex0#720", 0));

    let sender_handle = {
        let m = Arc::clone(&m);
        cir_trace::spawn("sender#796", move || sender(ch.0, m))
    };
    let receiver_handle = {
        let m = Arc::clone(&m);
        cir_trace::spawn("receiver#910", move || receiver(ch.1, m))
    };

    sender_handle.join().unwrap();
    receiver_handle.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
