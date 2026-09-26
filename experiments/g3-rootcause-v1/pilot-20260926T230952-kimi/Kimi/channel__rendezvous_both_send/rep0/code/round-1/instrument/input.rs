use std::sync::mpsc::{sync_channel, Receiver, SyncSender};
use std::thread;

fn s1(ch: SyncSender<i32>) {
    ch.send(1).unwrap();
}

fn r(ch: Receiver<i32>) -> i32 {
    let mut done = 0;
    done = ch.recv().unwrap();
    done
}

fn main() {
    let (tx, rx) = sync_channel::<i32>(0);

    let h_s1 = thread::spawn(move || s1(tx));
    let h_r = thread::spawn(move || r(rx));

    h_s1.join().unwrap();
    let done = h_r.join().unwrap();

    println!("DONE done={}", done);
}
