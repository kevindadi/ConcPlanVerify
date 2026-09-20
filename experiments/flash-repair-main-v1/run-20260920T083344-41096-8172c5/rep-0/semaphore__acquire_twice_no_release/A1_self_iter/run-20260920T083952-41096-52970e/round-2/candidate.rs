fn main() {
    let (tx, rx) = mpsc::sync_channel::<u32>(0);
    let s = std::sync::Arc::new(Sem::new());
    let s1 = std::sync::Arc::clone(&s);
    let tx1 = tx.clone();
    let w1 = std::thread::spawn(move || {
        s1.acq();
        s1.acq();
        s1.rel();
        s1.rel();
        let _ = tx1;
        let _ = rx;
    });
    ...
}
