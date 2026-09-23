let worker_b = thread::spawn(move || {
    let gb = mb2.lock().unwrap(); // s1: lock mtx_b
    sb2.release(); // s2: release sem_b
    sa2.acquire(); // s3: acquire sem_a (handshake)
    drop(gb); // release first lock before re-acquiring in global order
    let ga = ma2.lock().unwrap(); // s4: lock mtx_a (global order: a then b)
    let gb = mb2.lock().unwrap(); // s5: lock mtx_b
    drop(gb); // s6
    drop(ga); // s7
});
