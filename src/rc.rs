#![allow(warnings)]

use std::ptr::NonNull;

use crate::cell::Cell;

#[derive(Debug)]
struct RcInner<T> {
    value: T,
    refcount: Cell<usize>,
}

impl<T> RcInner<T> {
    /// Returns [`Self`] with refcount set to 1.
    const fn new(value: T) -> Self {
        Self {
            value,
            refcount: Cell::new(1),
        }
    }

    const fn increment(&self) {
        match self.count().checked_add(1) {
            Some(count) => self.set_count(count),
            None => panic!("Rc count overflown"),
        }
    }

    const fn decrement(&self) {
        let new = match self.count().checked_sub(1) {
            Some(count) => count,
            None => panic!("Rc count overflown"),
        };
        self.set_count(new);
    }

    const fn set_count(&self, count: usize) {
        self.refcount.set(count);
    }

    const fn count(&self) -> usize {
        self.refcount.get()
    }
}

#[derive(Debug)]
struct Rc<T> {
    inner: NonNull<RcInner<T>>,
}

impl<T> Rc<T> {
    pub fn new(value: T) -> Self {
        let inner = unsafe {
            let b = Box::new(RcInner::new(value));
            NonNull::new_unchecked(Box::into_raw(b))
        };
        Self { inner }
    }

    const fn increment(&self) {
        unsafe { self.inner.as_ref().increment() }
    }

    const fn count(&self) -> usize {
        unsafe { self.inner.as_ref().count() }
    }

    const fn decrement(&self) {
        unsafe { self.inner.as_ref().decrement() }
    }
}

impl<T> Clone for Rc<T> {
    fn clone(&self) -> Self {
        let inner = unsafe { self.inner.as_ref() };
        inner.increment();
        Self { inner: self.inner }
    }
}

impl<T> std::ops::Deref for Rc<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { &self.inner.as_ref().value }
    }
}

impl<T> Drop for Rc<T> {
    fn drop(&mut self) {
        self.decrement();
        if self.count() == 0 {
            std::mem::drop(unsafe { Box::from_raw(self.inner.as_ptr()) });
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn counts() {
        let r = Rc::new(crate::Vec::<String>::new());
        let cloned = Rc::clone(&r);
        assert_eq!(cloned.count(), 2);
        assert_eq!(r.count(), 2);
        std::mem::drop(r);
        assert_eq!(cloned.count(), 1);
    }

    #[test]
    fn counts_high() {
        let r = Rc::new(30);
        let exp = 50;

        let rs = (0..exp).map(|_| Rc::clone(&r)).collect::<Vec<_>>();
        assert_eq!(r.count(), exp + 1)
    }
}
