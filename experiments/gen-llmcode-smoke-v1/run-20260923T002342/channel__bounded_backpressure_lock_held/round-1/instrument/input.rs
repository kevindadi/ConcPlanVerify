use std::sync::{mpsc, Arc, Mutex};
use std::thread;

fn main() {
    let ch = mpsc::sync_channel::<i32>(1);
    let m = Arc::new(Mutex::new(()));

    let sender_ch = ch.0;
    let sender_m = Arc::clone(&m);
    let sender = thread::spawn(move || {
        {
            let _guard = sender_m.lock().unwrap();
        }
        sender_ch.send(1).unwrap();
        {
            let _guard = sender_m.lock().unwrap();
        }
        sender_ch.send(2).unwrap();
    });

    let receiver_ch = ch.1;
    let receiver_m = Arc::clone(&m);
    let receiver = thread::spawn(move || {
        {
            let _guard = receiver_m.lock().unwrap();
        }
        let _v1 = receiver_ch.recv().unwrap();
        {
            let _guard = receiver_m.lock().unwrap();
        }
        let _v2 = receiver_ch.recv().unwrap();
    });

    sender.join().unwrap();
    receiver.join().unwrap();

    println!("DONE done=1");
}
