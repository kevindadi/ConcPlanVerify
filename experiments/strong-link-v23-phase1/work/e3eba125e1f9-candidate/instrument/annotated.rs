mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::mpsc::{sync_channel, Receiver, SyncSender};
use std::sync::{Arc};
use std::thread;

fn sender(ch: SyncSender<i32>, m: Arc<Mutex<()>>) {
    for value in [1, 2] {
        {
            let _guard = m.lock().unwrap();
        }
        cir_trace::record("channel_send", "ch"); ch.send(value).unwrap();
    }
}

fn receiver(ch: Receiver<i32>, m: Arc<Mutex<()>>) {
    for expected in [1, 2] {
        {
            let _guard = m.lock().unwrap();
        }
        cir_trace::record("channel_recv", "ch"); let value = ch.recv().unwrap();
        assert_eq!(value, expected);
    }
}

fn main() { cir_trace::init();
    let (ch_sender, ch_receiver) = sync_channel(1);
    let m = Arc::new(Mutex::new_named("m_mutex0#613", ()));

    let sender_m = Arc::clone(&m);
    let sender_thread = cir_trace::spawn("sender#683", move || sender(ch_sender, sender_m));

    let receiver_m = Arc::clone(&m);
    let receiver_thread = cir_trace::spawn("receiver#799", move || receiver(ch_receiver, receiver_m));

    sender_thread.join().unwrap();
    receiver_thread.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
