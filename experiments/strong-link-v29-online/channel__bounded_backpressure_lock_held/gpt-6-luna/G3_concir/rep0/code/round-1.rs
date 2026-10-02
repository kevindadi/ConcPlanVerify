use std::sync::mpsc::{sync_channel, Receiver, SyncSender};
use std::sync::{Arc, Mutex};
use std::thread;

fn sender(m: Arc<Mutex<()>>, ch_tx: SyncSender<i32>) {
    {
        let _guard = m.lock().unwrap();
    }
    ch_tx.send(1).unwrap();

    {
        let _guard = m.lock().unwrap();
    }
    ch_tx.send(2).unwrap();
}

fn receiver(m: Arc<Mutex<()>>, ch_rx: Receiver<i32>) {
    {
        let _guard = m.lock().unwrap();
    }
    let _ = ch_rx.recv().unwrap();

    {
        let _guard = m.lock().unwrap();
    }
    let _ = ch_rx.recv().unwrap();
}

fn main() {
    let m = Arc::new(Mutex::new(()));
    let (ch_tx, ch_rx) = sync_channel(1);

    let sender_handle = thread::spawn(move || sender(m.clone(), ch_tx));
    let receiver_handle = thread::spawn(move || receiver(m, ch_rx));

    sender_handle.join().unwrap();
    receiver_handle.join().unwrap();

    println!("DONE done=1");
}
