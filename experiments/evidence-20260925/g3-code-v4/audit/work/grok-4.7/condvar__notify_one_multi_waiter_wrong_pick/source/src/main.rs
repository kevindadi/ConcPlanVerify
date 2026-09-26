use std::ffi::CString;
use std::os::raw::{c_char, c_int, c_uint, c_void};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::thread;

#[cfg(any(target_os = "macos", target_os = "ios"))]
const OFLAG_CREAT_EXCL: c_int = 0x0200 | 0x0800;
#[cfg(not(any(target_os = "macos", target_os = "ios")))]
const OFLAG_CREAT_EXCL: c_int = 0x40 | 0x80;

extern "C" {
    fn sem_open(name: *const c_char, oflag: c_int, ...) -> *mut c_void;
    fn sem_close(sem: *mut c_void) -> c_int;
    fn sem_unlink(name: *const c_char) -> c_int;
    fn sem_wait(sem: *mut c_void) -> c_int;
    fn sem_post(sem: *mut c_void) -> c_int;
}

struct Semaphore {
    ptr: *mut c_void,
}

unsafe impl Send for Semaphore {}
unsafe impl Sync for Semaphore {}

impl Semaphore {
    fn new(count: u32) -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let id = NEXT.fetch_add(1, Ordering::Relaxed);
        let name = CString::new(format!("/cpv{}{}", std::process::id(), id)).unwrap();
        let ptr = unsafe { sem_open(name.as_ptr(), OFLAG_CREAT_EXCL, 0o600 as c_int, count as c_uint) };
        if ptr.is_null() || (ptr as isize) == -1 {
            panic!("sem_open failed");
        }
        unsafe {
            sem_unlink(name.as_ptr());
        }
        Semaphore { ptr }
    }
}

impl Drop for Semaphore {
    fn drop(&mut self) {
        unsafe {
            sem_close(self.ptr);
        }
    }
}

#[inline(never)]
fn sem_acquire(g: &Semaphore) {
    let rc = unsafe { sem_wait(g.ptr) };
    if rc != 0 {
        panic!("sem_wait failed");
    }
}

#[inline(never)]
fn sem_release(g: &Semaphore) {
    let rc = unsafe { sem_post(g.ptr) };
    if rc != 0 {
        panic!("sem_post failed");
    }
}

fn w1(m: &Mutex<()>, cv: &Condvar, g12: &Semaphore, gN: &Semaphore) {
    let guard = m.lock().unwrap();
    sem_release(g12);
    let guard = cv.wait(guard).unwrap();
    drop(guard);
    sem_release(gN);
}

fn w2(m: &Mutex<()>, cv: &Condvar, g12: &Semaphore, gN: &Semaphore) {
    let guard = m.lock().unwrap();
    sem_release(g12);
    let guard = cv.wait(guard).unwrap();
    drop(guard);
    sem_release(gN);
}

fn notifier(m: &Mutex<()>, cv: &Condvar, g12: &Semaphore, gN: &Semaphore) {
    sem_acquire(g12);
    sem_acquire(g12);
    let guard = m.lock().unwrap();
    cv.notify_one();
    cv.notify_one();
    drop(guard);
    sem_acquire(gN);
    sem_acquire(gN);
}

fn main() {
    let m = Arc::new(Mutex::new(()));
    let cv = Arc::new(Condvar::new());
    let g12 = Arc::new(Semaphore::new(0));
    let gN = Arc::new(Semaphore::new(0));

    let m1 = Arc::clone(&m);
    let cv1 = Arc::clone(&cv);
    let g12_1 = Arc::clone(&g12);
    let gN_1 = Arc::clone(&gN);
    let h1 = thread::spawn(move || w1(&m1, &cv1, &g12_1, &gN_1));

    let m2 = Arc::clone(&m);
    let cv2 = Arc::clone(&cv);
    let g12_2 = Arc::clone(&g12);
    let gN_2 = Arc::clone(&gN);
    let h2 = thread::spawn(move || w2(&m2, &cv2, &g12_2, &gN_2));

    let mn = Arc::clone(&m);
    let cvn = Arc::clone(&cv);
    let g12n = Arc::clone(&g12);
    let gNn = Arc::clone(&gN);
    let hn = thread::spawn(move || notifier(&mn, &cvn, &g12n, &gNn));

    h1.join().unwrap();
    h2.join().unwrap();
    hn.join().unwrap();

    println!("DONE waiters=0");
}
