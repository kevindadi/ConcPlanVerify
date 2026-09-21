fn main() {
    let shared = Arc::new(Mutex::new(0u32));
    let chan = Arc::new(Rendezvous::new());

    let shared_s = Arc::clone(&shared);
    let chan_s = Arc::clone(&chan);
    let sender = thread::spawn(move || {
        {
            let mut g = shared_s.lock().unwrap();
            *g += 1;
        }
        chan_s.send(1u32);
    });

    let shared_r = Arc::clone(&shared);
    let chan_r = Arc::clone(&chan);
    let receiver = thread::spawn(move || {
        {
            let mut g = shared_r.lock().unwrap();
            *g += 1;
        }
        let v = chan_r.recv();
        v
    });

    sender.join().unwrap();
    let done = receiver.join().unwrap();
    println!("DONE done={}", done);
}
