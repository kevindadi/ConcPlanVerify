fn acquire(&self) {
    let mut permits = self.permits.lock().unwrap();
    while *permits == 0 {
        permits = self.cv.wait(permits).unwrap();
    }
    *permits -= 1;
}

fn release(&self) {
    *self.permits.lock().unwrap() += 1;
    self.cv.notify_one();
}
