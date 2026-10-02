mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{mpsc::{sync_channel, Receiver, SyncSender};
use std::thread;

fn sender(ch: SyncSender<usize>, m: Arc<Mutex<usize>>) {
    for value in [1, 2] {
        {
            let mut count = m.lock().unwrap();
            *count += 1;
        } // Release m before sending, which may wait.
        cir_trace::record("channel_send", "ch"); ch.send(value).unwrap();
    }
}

fn receiver(ch: Receiver<usize>, m: Arc<Mutex<usize>>) {
    for expected in [1, 2] {
        cir_trace::record("channel_recv", "ch"); let value = ch.recv().unwrap(); // Wait without holding m.
        assert_eq!(value, expected);

        let mut count = m.lock().unwrap();
        *count += 1;
    }
}

fn main() { cir_trace::init();
    let m = Arc::new(Mutex::new_named("m_mutex0#668", 0));
    let ch = sync_channel::<usize>(1);
    let (sender_ch, receiver_ch) = ch;

    let sender_thread = {
        let m = Arc::clone(&m);
        cir_trace::spawn("sender#822", move || sender(sender_ch, m))
    };
    let receiver_thread = {
        let m = Arc::clone(&m);
        cir_trace::spawn("receiver#941", move || receiver(receiver_ch, m))
    };

    sender_thread.join().unwrap();
    receiver_thread.join().unwrap();
    assert_eq!(*m.lock().unwrap(), 4);

    println!("DONE done=1");
 cir_trace::finish();}
