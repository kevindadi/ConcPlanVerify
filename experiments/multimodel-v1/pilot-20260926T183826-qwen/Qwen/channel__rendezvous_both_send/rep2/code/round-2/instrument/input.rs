use std::sync::mpsc::{channel, Sender, Receiver};
use std::thread;

fn s1(tx: Sender<i32>) {
    tx.send(1).unwrap();
}

fn r(rx: Receiver<i32>) {
    let _val = rx.recv().unwrap();
}

fn main() {
    let (tx, rx): (Sender<i32>, Receiver<i32>) = channel();

    let s1_handle = thread::spawn(move || {
        s1(tx);
    });

    let r_handle = thread::spawn(move || {
        r(rx);
    });

    s1_handle.join().unwrap();
    r_handle.join().unwrap();

    println!("DONE done=1");
}
