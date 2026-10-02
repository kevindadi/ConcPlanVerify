mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{mpsc::{sync_channel, Receiver, SyncSender};
use std::thread;

fn sender(ch: SyncSender<u32>, m: Arc<Mutex<()>>) {
    for value in [1, 2] {
        let guard = m.lock().unwrap();
        drop(guard);

        cir_trace::record("channel_send", "ch"); ch.send(value).unwrap();
    }
}

fn receiver(ch: Receiver<u32>, m: Arc<Mutex<()>>) {
    for _ in 0..2 {
        let guard = m.lock().unwrap();
        drop(guard);

        cir_trace::record("channel_recv", "ch"); ch.recv().unwrap();
    }
}

fn main() { cir_trace::init();
    let ch = sync_channel::<u32>(1);
    let (ch_sender, ch_receiver) = ch;
    let m = Arc::new(Mutex::new_named("m_mutex0#569", ()));

    let sender_handle = {
        let m = Arc::clone(&m);
        cir_trace::spawn("sender#646", move || sender(ch_sender, m))
    };

    let receiver_handle = cir_trace::spawn("receiver#724", move || receiver(ch_receiver, m));

    sender_handle.join().unwrap();
    receiver_handle.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
