use std::sync::mpsc::sync_channel;
use std::thread;

type Int = i32;

fn s1(ch: std::sync::mpsc::Sender<Int>) {
    ch.send(1).unwrap();
}

fn r(ch: std::sync::mpsc::Receiver<Int>) {
    let mut v: Int = 0;
    v = ch.recv().unwrap();
}

fn main() {
    let (ch, ch_r) = sync_channel::<Int>(0);

    let h_s1 = thread::spawn(move || s1(ch));
    let h_r = thread::spawn(move || r(ch_r));

    h_s1.join().unwrap();
    h_r.join().unwrap();

    println!("DONE done=1");
}
