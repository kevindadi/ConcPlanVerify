mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{mpsc::{sync_channel, Receiver, SyncSender}, Arc};

fn sender(ch: SyncSender<u8>, m: Arc<Mutex<usize>>) {
    for value in [1, 2] {
        // Do not hold m while send may block on a full channel.
        cir_trace::record("channel_send", "ch"); ch.send(value).expect("receiver disconnected");

        let mut count = m.lock().expect("lock poisoned");
        *count += 1;
    }
}

fn receiver(ch: Receiver<u8>, m: Arc<Mutex<usize>>) {
    for expected in [1, 2] {
        // Do not hold m while recv may block on an empty channel.
        cir_trace::record("channel_recv", "ch"); let value = ch.recv().expect("sender disconnected");
        assert_eq!(value, expected);

        let mut count = m.lock().expect("lock poisoned");
        *count += 1;
    }
}

fn main() { cir_trace::init();
    let m = Arc::new(Mutex::new_named("m_mutex0#752", 0));
    let ch = sync_channel::<u8>(1);
    let (tx, rx) = ch;

    let sender_handle = {
        let m = Arc::clone(&m);
        cir_trace::spawn("sender#887", move || sender(tx, m))
    };
    let receiver_handle = {
        let m = Arc::clone(&m);
        cir_trace::spawn("receiver#1004", move || receiver(rx, m))
    };

    sender_handle.join().expect("sender panicked");
    receiver_handle.join().expect("receiver panicked");
    assert_eq!(*m.lock().expect("lock poisoned"), 4);

    println!("DONE done=1");
 cir_trace::finish();}
