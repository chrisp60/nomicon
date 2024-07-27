#![allow(unused)]

use std::{
    cell::UnsafeCell,
    ptr::NonNull,
    sync::atomic::{AtomicUsize, Ordering},
};

pub struct Arc<T> {
    inner: NonNull<ArcInner<T>>,
}

impl<T> Arc<T> {
    pub fn new(value: T) -> Self {
        Self {
            inner: unsafe {
                let ptr = Box::into_raw(Box::new(ArcInner::new(value)));
                NonNull::new_unchecked(ptr)
            },
        }
    }

    fn increment(&self) {
        unsafe { self.inner.as_ref() }.increment()
    }

    fn decrement(&self) {
        unsafe { self.inner.as_ref() }.decrement()
    }

    fn count(&self) -> usize {
        unsafe { self.inner.as_ref() }.count.load(Ordering::Relaxed)
    }
}

unsafe impl<T: Send> Send for Arc<T> {}
unsafe impl<T: Sync> Sync for Arc<T> {}

impl<T> std::ops::Deref for Arc<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { &self.inner.as_ref().value }
    }
}

impl<T> Clone for Arc<T> {
    fn clone(&self) -> Self {
        self.increment();
        Self { inner: self.inner }
    }
}

impl<T> Drop for Arc<T> {
    fn drop(&mut self) {
        self.decrement();
        if self.count() == 0 {
            std::mem::drop(unsafe { Box::from_raw(self.inner.as_ptr()) })
        }
    }
}

struct ArcInner<T> {
    value: T,
    count: AtomicUsize,
}

impl<T> ArcInner<T> {
    fn new(value: T) -> Self {
        Self {
            value,
            count: AtomicUsize::new(1),
        }
    }

    fn increment(&self) {
        self.count.fetch_add(1, Ordering::AcqRel);
    }

    fn decrement(&self) {
        self.count.fetch_sub(1, Ordering::AcqRel);
    }
}

unsafe impl<T: Send> Send for ArcInner<T> {}
unsafe impl<T: Sync> Sync for ArcInner<T> {}

#[cfg(test)]
mod test {
    use std::{ops::Deref, thread};

    use super::*;

    #[test]
    fn counts() {
        let arc = Arc::new(String::from("Hello, World"));
        let cloned = Arc::clone(&arc);
        assert_eq!(arc.count(), 2);
        std::mem::drop(arc);
        assert_eq!(cloned.count(), 1);
    }

    #[test]
    fn threads() {
        let arc = Arc::new(String::from("Hello, World"));
        let threads = (0..100)
            .map(|_| {
                let another = Arc::clone(&arc);
                thread::spawn(move || {
                    assert_eq!(another.deref(), "Hello, World");
                    std::mem::drop(another);
                })
            })
            .collect::<Vec<_>>();

        for thread in threads {
            thread.join().unwrap();
        }

        assert_eq!(arc.count(), 1)
    }
}
