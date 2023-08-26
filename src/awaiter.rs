use std::sync::{Arc, Condvar, Mutex};

#[derive(Clone)]
pub struct Awaiter(Arc<(Mutex<bool>, Condvar)>);

impl Awaiter {
    pub fn new() -> Self {
        Self(Arc::new((Mutex::new(false), Condvar::new())))
    }

    pub fn unlock(&self) {
        let (lock, condvar) = self.0.as_ref();
        let mut stopped = lock.lock().unwrap();
        *stopped = true;
        condvar.notify_one();
    }

    pub fn wait(&self) {
        let (lock, condvar) = self.0.as_ref();
        let mut stopped = lock.lock().unwrap();
        while !*stopped {
            stopped = condvar.wait(stopped).unwrap();
        }
    }
}

impl Default for Awaiter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::thread;

    use super::*;

    #[test]
    fn test_wait_until_unlocked() {
        let awaiter = Awaiter::new();
        let awaiter_in_thread = awaiter.clone();
        let value = Arc::new(AtomicU32::new(0));
        let value_in_thread = Arc::clone(&value);

        thread::spawn(move || {
            while value_in_thread.load(Ordering::Acquire) < 5 {
                thread::yield_now();
                value_in_thread.fetch_add(1, Ordering::AcqRel);
            }
            awaiter_in_thread.unlock();
        });

        awaiter.wait();
        assert_eq!(value.load(Ordering::Acquire), 5);
    }
}
