#![allow(dead_code)]

use std::{
    alloc::{alloc, dealloc, handle_alloc_error, realloc, Layout},
    ptr::{self, NonNull},
};

#[derive(Debug)]
pub struct Vec<T> {
    ptr: NonNull<T>,
    cap: usize,
    len: usize,
}

impl<T> Vec<T> {
    pub fn new() -> Self {
        assert!(std::mem::size_of::<T>() != 0, "unable to handle ZSTs");
        Vec {
            ptr: NonNull::dangling(),
            cap: 0,
            len: 0,
        }
    }

    fn grow(&mut self) {
        let (new_cap, new_layout) = if self.cap == 0 {
            // prepare a layout alligned to [T; 1]
            (1, Layout::array::<T>(1).expect("not overflown"))
        } else {
            // double the current capacity and
            // prepare a layout aligned to [T; new_cap]
            let new_cap = self.cap * 2;
            let new_layout = Layout::array::<T>(new_cap).expect("not overflown");
            (new_cap, new_layout)
        };

        // All allocations are restrictred to isize::MAX bytes
        assert!(
            new_layout.size() <= isize::MAX as usize,
            "allocation too large"
        );

        let new_ptr = if self.cap == 0 {
            // we are requesting a new allocation so no ptr needs to be provided
            // only the layout is supplied to the allocator which then
            // returns an Option<*mut u8>
            unsafe { alloc(new_layout) }
        } else {
            // it is UB to realloc using a layout that is different than
            // the layout provided on alloc
            let old_layout = Layout::array::<T>(self.cap).expect("not overflown");
            // Cast our current ptr to a raw ptr (*mut u8)
            let old_ptr = self.ptr.as_ptr() as *mut u8;
            // Feeding the old ptr to realloc with the *old_layout* and
            // the size of a new_layout.
            //
            // The size is provided from the layout due to the layout being
            // alligned to T. The allignment T != allignment of U.
            unsafe { realloc(old_ptr, old_layout, new_layout.size()) }
        };

        // Failed allocations return None, in which case we abort
        //
        // aborting is prefferred due to unwinding generally requiring addt
        // allocs.
        //
        // If the ptr is good, update our current ptr.
        self.ptr = match NonNull::new(new_ptr as *mut T) {
            Some(ptr) => ptr,
            None => handle_alloc_error(new_layout),
        };

        // Finally, update our cap.
        self.cap = new_cap;
    }

    pub fn push(&mut self, elem: T) {
        // grow when len is equal to capacity
        if self.len == self.cap {
            self.grow()
        };

        unsafe {
            let dst = self.ptr.as_ptr().add(self.len);
            let src = elem;
            ptr::write(dst, src)
        }

        // OOM before overflow
        self.len += 1;
    }

    pub fn pop(&mut self) -> Option<T> {
        if self.len == 0 {
            None
        } else {
            self.len -= 1;
            unsafe { Some(ptr::read(self.ptr.as_ptr().add(self.len))) }
        }
    }
}

impl<T> std::ops::Deref for Vec<T> {
    type Target = [T];

    fn deref(&self) -> &Self::Target {
        unsafe { std::slice::from_raw_parts(self.ptr.as_ptr(), self.len) }
    }
}

impl<T> Drop for Vec<T> {
    fn drop(&mut self) {
        if self.cap != 0 {
            while self.pop().is_some() {}
            let layout = Layout::array::<T>(self.cap).unwrap();
            unsafe {
                dealloc(self.ptr.as_ptr() as *mut u8, layout);
            }
        }
    }
}

impl<T> Default for Vec<T> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn init_vec() {
        Vec::<u8>::new();
    }

    #[test]
    fn push_and_pop() {
        let mut v = Vec::new();
        v.push(1);
        v.push(2);
        assert_eq!(v.pop(), Some(2));
        assert_eq!(v.pop(), Some(1));
        assert_eq!(v.pop(), None);
    }
}
