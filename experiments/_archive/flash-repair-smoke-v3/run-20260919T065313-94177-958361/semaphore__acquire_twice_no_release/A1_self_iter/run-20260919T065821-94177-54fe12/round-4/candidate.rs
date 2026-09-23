fn rel(&self) {
    let me = std::thread::current().id();
    let mut c = self.n.lock().unwrap();
    if c.owner == Some(me) {
        c.count -= 1;
        if c.count == 0 {
            c.owner = None;
            c.permits += 1;
            self.cv.notify_one();
        }
    }
}
