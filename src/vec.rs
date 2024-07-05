use std::{
    alloc::{alloc, handle_alloc_error, realloc, Layout},
    mem::ManuallyDrop,
    ptr::NonNull,
};

pub struct Vec<T> {
    ptr: NonNull<T>,
    len: usize,
    cap: usize,
}

impl<T> Vec<T> {
    pub const fn new() -> Self {
        const {
            assert!(
                std::mem::size_of::<T>() != 0,
                "Zero sized types are not supported"
            )
        }
        Self {
            ptr: NonNull::dangling(),
            len: 0,
            cap: 0,
        }
    }

    fn grow(&mut self) {
        let (new_cap, new_layout) = if self.cap == 0 {
            (1, Layout::array::<T>(1).unwrap())
        } else {
            let new_cap = self.cap * 2;
            (new_cap, Layout::array::<T>(new_cap).unwrap())
        };

        assert!(
            new_layout.size() < isize::MAX as usize,
            "allocations cannot exceed isize MAX"
        );

        let ptr = if self.cap == 0 {
            unsafe { alloc(new_layout) }
        } else {
            let old_layout = Layout::array::<T>(self.cap).unwrap();
            let ptr = self.ptr.as_ptr() as *mut u8;
            unsafe { realloc(ptr, old_layout, new_layout.size()) }
        };
        self.ptr = match NonNull::new(ptr as *mut T) {
            Some(ptr) => ptr,
            None => handle_alloc_error(new_layout),
        };
        self.cap = new_cap;
    }

    pub fn push(&mut self, item: T) {
        if self.len == self.cap {
            self.grow();
        }
        unsafe {
            let dst = self.ptr.as_ptr().add(self.len);
            std::ptr::write(dst, item)
        }
        self.len += 1;
    }

    pub fn pop(&mut self) -> Option<T> {
        if self.len == 0 {
            return None;
        }
        self.len -= 1;

        Some(unsafe {
            let src = self.ptr.as_ptr().add(self.len);
            std::ptr::read(src)
        })
    }

    pub fn is_empty(&self) -> bool {
        self.len().eq(&0)
    }

    pub const fn len(&self) -> usize {
        self.len
    }
}

impl<T> Default for Vec<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> Drop for Vec<T> {
    fn drop(&mut self) {
        if self.cap != 0 {
            while self.pop().is_some() {}
            unsafe {
                // SAFETY
                // self.cap is not zero, so we have allocated
                // self.cap is updated alongside the side of our allocation.
                let ptr = self.ptr.as_ptr() as *mut u8;
                let layout = Layout::array::<T>(self.cap).unwrap();
                std::alloc::dealloc(ptr, layout)
            }
        }
    }
}

pub struct IntoIter<T> {
    buf: NonNull<T>,
    cap: usize,
    start: *const T,
    end: *const T,
}

impl<T> IntoIterator for Vec<T> {
    type IntoIter = IntoIter<T>;
    type Item = T;

    fn into_iter(self) -> Self::IntoIter {
        let s = ManuallyDrop::new(self);
        let len = s.len;
        let cap = s.cap;
        let ptr = s.ptr;
        let buf = s.ptr;
        IntoIter {
            buf,
            cap,
            start: ptr.as_ptr(),
            end: if cap == 0 {
                ptr.as_ptr()
            } else {
                unsafe { ptr.as_ptr().add(len) }
            },
        }
    }
}

impl<T> Iterator for IntoIter<T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        if self.start == self.end {
            None
        } else {
            unsafe {
                let item = std::ptr::read(self.start);
                self.start = self.start.offset(1);
                Some(item)
            }
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        const {
            assert!(std::mem::size_of::<T>() != 0);
        }
        unsafe {
            // SAFEY
            // * self.end and self.start are derived from the same object
            // * Allocation never exceeds isize::MAX
            // * both in bounds
            let len = self.start.offset_from(self.end) as usize;
            (len, Some(len))
        }
    }
}

impl<T> Drop for IntoIter<T> {
    fn drop(&mut self) {
        if self.cap != 0 {
            for _ in &mut *self {}
            unsafe {
                std::alloc::dealloc(
                    self.buf.as_ptr() as *mut u8,
                    Layout::array::<T>(self.cap).unwrap(),
                )
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn push_and_pop() {
        let mut b = Vec::<u8>::new();
        b.push(1);
        b.push(3);
        assert_eq!(b.len(), 2);
        assert_eq!(b.pop(), Some(3));
        assert_eq!(b.pop(), Some(1));
        assert_eq!(b.pop(), None);
        assert_eq!(b.len(), 0);
    }

    #[test]
    fn iter() {
        let mut b = Vec::<u8>::new();
        b.push(1);
        b.push(2);
        b.push(3);
        b.push(4);
        let mut iter = b.into_iter();
        assert_eq!(iter.next(), Some(1));
        assert_eq!(iter.next(), Some(2));
        assert_eq!(iter.next(), Some(3));
        assert_eq!(iter.next(), Some(4));
        assert_eq!(iter.next(), None);
    }
}
