use std::sync::{
    mpsc::{sync_channel, Receiver, SyncSender},
    Arc, Mutex,
};
use std::thread;

fn sender(ch: SyncSender<usize>, m: Arc<Mutex<usize>>) {
    for value in [1, 2] {
        {
            let mut count = m.lock().unwrap();
            *count += 1;
        } // Release m before a send that may wait.

        ch.send(value).unwrap();
    }
}

fn receiver(ch: Receiver<usize>, m: Arc<Mutex<usize>>) {
    for expected in [1, 2] {
        let value = ch.recv().unwrap(); // Wait without holding m.
        assert_eq!(value, expected);

        {
            let mut count = m.lock().unwrap();
            *count += 1;
        }
    }
}

fn main() {
    let m = Arc::new(Mutex::new(0));
    let (sender_ch, receiver_ch) = sync_channel::<usize>(1);

    let sender_thread = {
        let m = Arc::clone(&m);
        thread::spawn(move || sender(sender_ch, m))
    };

    let receiver_thread = {
        let m = Arc::clone(&m);
        thread::spawn(move || receiver(receiver_ch, m))
    };

    sender_thread.join().unwrap();
    receiver_thread.join().unwrap();
    assert_eq!(*m.lock().unwrap(), 4);

    println!("DONE done=1");
}
