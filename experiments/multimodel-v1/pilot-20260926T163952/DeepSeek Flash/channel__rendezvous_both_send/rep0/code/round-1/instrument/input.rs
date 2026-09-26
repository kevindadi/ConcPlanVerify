use std::sync::mpsc::{channel, Sender, Receiver};
use std::thread;

fn send(tx: &Sender<i32>, v: i32) {
    tx.send(v).unwrap();
}

fn recv(rx: &Receiver<i32>) -> i32 {
    let mut got: i32 = 0;
    got = rx.recv().unwrap();
    got
}

fn s1(tx: Sender<i32>) {
    send(&tx, 1);
}

fn r(rx: Receiver<i32>) {
    recv(&rx);
}

fn main() {
    let (tx, rx) = channel::<i32>();

    let h1 = thread::spawn(move || {
        s1(tx);
    });

    let h2 = thread::spawn(move || {
        r(rx);
    });

    h1.join().unwrap();
    h2.join().unwrap();

    println!("DONE done=1");
}
