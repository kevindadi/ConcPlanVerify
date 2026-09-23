use std::sync::mpsc::{sync_channel, Receiver, SyncSender};
use std::thread;

struct Channels {
    ch1_tx: SyncSender<i32>,
    ch1_rx: Receiver<i32>,
    ch2_tx: SyncSender<i32>,
    ch2_rx: Receiver<i32>,
}

fn s(ch1_tx: SyncSender<i32>, ch2_rx: Receiver<i32>) {
    ch1_tx.send(1).unwrap();
    let _ack: i32 = ch2_rx.recv().unwrap();
}

fn r(ch1_rx: Receiver<i32>, ch2_tx: SyncSender<i32>) {
    let _v: i32 = ch1_rx.recv().unwrap();
    ch2_tx.send(1).unwrap();
}

fn main() {
    let (ch1_tx, ch1_rx) = sync_channel::<i32>(0);
    let (ch2_tx, ch2_rx) = sync_channel::<i32>(0);

    let ch = Channels {
        ch1_tx,
        ch1_rx,
        ch2_tx,
        ch2_rx,
    };

    let s_handle = thread::spawn(move || {
        s(ch.ch1_tx, ch.ch2_rx);
    });

    let r_handle = thread::spawn(move || {
        r(ch.ch1_rx, ch.ch2_tx);
    });

    s_handle.join().unwrap();
    r_handle.join().unwrap();

    println!("DONE done=1");
}
