use std::sync::mpsc::{sync_channel, Receiver, SyncSender};
use std::sync::{Arc, Mutex};
use std::thread;

fn s(ch1_tx: SyncSender<usize>, ch2_rx: Receiver<usize>, shared: Arc<Mutex<usize>>) {
    {
        let mut value = shared.lock().expect("shared lock poisoned");
        *value += 1;
    } // Release the lock before waiting on either channel.

    ch1_tx.send(42).expect("receiver exited");
    let reply = ch2_rx.recv().expect("receiver exited");

    {
        let mut value = shared.lock().expect("shared lock poisoned");
        *value += reply;
    }
}

fn r(ch1_rx: Receiver<usize>, ch2_tx: SyncSender<usize>, shared: Arc<Mutex<usize>>) {
    {
        let mut value = shared.lock().expect("shared lock poisoned");
        *value += 1;
    } // Release the lock before waiting on either channel.

    let received = ch1_rx.recv().expect("sender exited");

    {
        let mut value = shared.lock().expect("shared lock poisoned");
        *value += received;
    } // Release the lock before the rendezvous send.

    ch2_tx.send(received).expect("sender exited");
}

fn main() {
    let ch1 = sync_channel::<usize>(0);
    let ch2 = sync_channel::<usize>(0);
    let (ch1_tx, ch1_rx) = ch1;
    let (ch2_tx, ch2_rx) = ch2;

    let shared = Arc::new(Mutex::new(0));

    let s_shared = Arc::clone(&shared);
    let s_handle = thread::spawn(move || s(ch1_tx, ch2_rx, s_shared));

    let r_shared = Arc::clone(&shared);
    let r_handle = thread::spawn(move || r(ch1_rx, ch2_tx, r_shared));

    s_handle.join().expect("sender thread panicked");
    r_handle.join().expect("receiver thread panicked");

    println!("DONE done=1");
}
