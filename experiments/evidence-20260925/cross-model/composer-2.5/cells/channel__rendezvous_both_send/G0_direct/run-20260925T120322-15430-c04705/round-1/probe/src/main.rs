use concir_sync::Semaphore;
use std::sync::mpsc;
use std::thread;

fn s1(ch: mpsc::SyncSender<i32>) {
    ch.send(1).unwrap();
}

fn r(ch: mpsc::Receiver<i32>) {
    let _ = ch.recv().unwrap();
}

fn main() {
    let (ch_tx, ch_rx) = mpsc::sync_channel::<i32>(0);

    let s1_handle = thread::spawn(move || s1(ch_tx));
    let r_handle = thread::spawn(move || r(ch_rx));

    s1_handle.join().unwrap();
    r_handle.join().unwrap();

    println!("DONE done=1");
}
