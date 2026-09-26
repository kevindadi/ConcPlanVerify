use std::sync::mpsc::{channel, Sender, Receiver};
use std::thread;

fn s1(tx: Sender<i32>) {
    tx.send(1).unwrap();
}

fn r(rx: Receiver<i32>) -> i32 {
    let v = rx.recv().unwrap();
    v
}

fn main() {
    let (tx, rx) = channel::<i32>();

    let h1 = thread::spawn(move || s1(tx));
    let h2 = thread::spawn(move || r(rx));

    h1.join().unwrap();
    let done = h2.join().unwrap();

    println!("DONE done={}", done);
}
