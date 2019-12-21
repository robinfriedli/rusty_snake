use std::cell::UnsafeCell;
use completable_future::CompletableFuture;
use futures::Future;

pub struct FutureHolder {
    future: UnsafeCell<Option<CompletableFuture<i32, ()>>>
}

impl FutureHolder {
    pub const fn new() -> FutureHolder {
        FutureHolder { future: UnsafeCell::new(None) }
    }

    pub fn prepare(&self) {
        unsafe {
            self.future.get().replace(Some(CompletableFuture::<i32, ()>::new()));
        }
    }

    /// Replaces the current Option with None and transfers ownership of the previous value to the caller
    pub fn consume(&self) -> Option<CompletableFuture<i32, ()>> {
        unsafe {
            self.future.get().replace(None)
        }
    }

    /// wait for the value to be completed in another thread, blocking the current thread, this consumes
    /// the current future
    pub fn wait(&self) -> Option<i32> {
        let consumed_val = self.consume();
        match consumed_val {
            Some(fut) => {
                let val = fut.wait().unwrap();
                return Some(val);
            }
            None => {
                return None;
            }
        };
    }

    pub fn is_empty(&self) -> bool {
        unsafe {
            self.future.get().read().is_none()
        }
    }

    pub fn complete_optional(&self, val: i32) {
        unsafe {
            match self.future.get().read() {
                Some(fut) => {
                    fut.signal().complete(val);
                }
                None => {}
            };
        }
    }
}