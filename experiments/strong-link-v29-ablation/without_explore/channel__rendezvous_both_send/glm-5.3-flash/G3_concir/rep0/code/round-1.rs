use std::sync::mpsc::sync_channel;
use std::thread;

fn s1(ch_tx: std::sync::mpsc::SyncSender<i32>) {
    ch_tx.send(1).expect("send failed");
}

fn r(ch_rx: std::sync::mpsc::Receiver<i32>) {
    let _ = ch_rx.recv().expect("recv failed");
}

fn main() {
    let (ch_tx, ch_rx) = sync_channel::<i32>(0);

    let s1_handle = thread::spawn(move || s1(ch_tx));
    let r_handle = thread::spawn(move || r(ch_rx));

    s1_handle.join().expect("s1 panicked");
    r_handle.join().expect("r panicked");

    println!("DONE done=1");
}
