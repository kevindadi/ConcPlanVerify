let worker_b = thread::spawn(move || {
    sb2.release(); // s1: release sem_b (handshake)
    sa2.acquire(); // s2: acquire sem_a (handshake)
    let ga = ma2.lock().unwrap(); // s3: lock mtx_a
    let gb = mb2.lock().unwrap(); // s4: lock mtx_b
    drop(gb); // s5
    drop(ga); // s6
});
