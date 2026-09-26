use std::sync::mpsc::{sync_channel, Receiver, SyncSender};
use std::thread;

fn s1(ch: SyncSender<i32>) {
    ch.send(1).unwrap();
}

fn r(ch: Receiver<i32>) {
    let _ = ch.recv().unwrap();
}

fn main() {
    let (ch_sender, ch_receiver) = sync_channel(0);

    let s1_thread = thread::spawn(move || s1(ch_sender));
    let r_thread = thread::spawn(move || r(ch_receiver));

    s1_thread.join().unwrap();
    r_thread.join().unwrap();

    println!("DONE done=1");
}
