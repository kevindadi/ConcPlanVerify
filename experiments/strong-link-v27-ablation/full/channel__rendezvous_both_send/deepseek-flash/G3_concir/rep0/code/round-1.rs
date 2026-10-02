use std::sync::mpsc::{sync_channel, SyncSender};
use std::thread;

fn main() {
    let ch = sync_channel::<i32>(0);

    let s1_handle = {
        let ch_sender: SyncSender<i32> = ch.0.clone();
        thread::spawn(move || s1(ch_sender))
    };

    let r_handle = {
        let ch_receiver = ch.1;
        thread::spawn(move || r(ch_receiver))
    };

    s1_handle.join().unwrap();
    r_handle.join().unwrap();

    println!("DONE done=1");
}

fn s1(ch: SyncSender<i32>) {
    ch.send(1).unwrap();
}

fn r(ch: std::sync::mpsc::Receiver<i32>) {
    let v: i32 = ch.recv().unwrap();
    let _ = v;
}
