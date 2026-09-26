use std::sync::mpsc::sync_channel;
use std::thread;

fn s1(ch: std::sync::mpsc::Sender<i32>) {
    ch.send(1).unwrap();
}

fn r(ch: std::sync::mpsc::Receiver<i32>) {
    let v = ch.recv().unwrap();
    let _ = v;
}

fn main() {
    let ch = sync_channel::<i32>(0);
    let ch_tx = ch.0;
    let ch_rx = ch.1;

    let h_s1 = thread::spawn(move || s1(ch_tx));
    let h_r = thread::spawn(move || r(ch_rx));

    h_s1.join().unwrap();
    h_r.join().unwrap();

    println!("DONE done=1");
}
