use std::{
    alloc::{alloc, dealloc, handle_alloc_error, realloc, Layout},
    mem,
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
        const { assert!(mem::size_of::<T>() != 0, "Zero sized types not supported") }
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
            let old_layout = Layout::array::<T>(self.cap).expect("not overflown");
            let old_ptr = self.ptr.as_ptr().cast();
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
            unsafe {
                let src = self.ptr.as_ptr().add(self.len);
                Some(ptr::read(src))
            }
        }
    }

    pub fn insert(&mut self, index: usize, elem: T) {
        // inserting at the end of len is just pushing
        assert!(index <= self.len, "index out of bounds");
        if self.cap == self.len {
            self.grow()
        }
        unsafe {
            ptr::copy(
                // inserting at index 2 shifts the elements at 2.. to 3..
                self.ptr.as_ptr().add(index),
                self.ptr.as_ptr().add(index + 1),
                self.len - index,
            );
            ptr::write(self.ptr.as_ptr().add(index), elem);
            self.len += 1;
        }
    }

    pub fn remove(&mut self, index: usize) -> T {
        assert!(index < self.len, "index out of bounds");
        unsafe {
            self.len -= 1;
            let result = ptr::read(self.ptr.as_ptr().add(index));
            ptr::copy(
                self.ptr.as_ptr().add(index + 1),
                self.ptr.as_ptr().add(index),
                self.len - index,
            );
            result
        }
    }
}

impl<T> std::ops::Deref for Vec<T> {
    type Target = [T];

    fn deref(&self) -> &Self::Target {
        unsafe { std::slice::from_raw_parts(self.ptr.as_ptr(), self.len) }
    }
}

impl<T> std::ops::DerefMut for Vec<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { std::slice::from_raw_parts_mut(self.ptr.as_ptr(), self.len) }
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

    macro_rules! mut_vec {
        ($i:ident) => {
            let mut $i = Vec::<u8>::new();
        };
    }

    #[test]
    fn push_and_pop() {
        mut_vec!(v);
        v.push(1);
        v.push(2);
        assert_eq!(v.pop(), Some(2));
        assert_eq!(v.pop(), Some(1));
        assert_eq!(v.pop(), None);
    }

    #[test]
    fn inserts() {
        mut_vec!(v);
        v.push(1);
        v.insert(0, 3);
        assert_eq!(v.pop(), Some(1));
        assert_eq!(v.pop(), Some(3));
        assert_eq!(v.pop(), None);
    }

    #[test]
    #[should_panic = "index out of bounds"]
    fn out_of_bound_panic() {
        mut_vec!(v);
        v.remove(0);
    }

    #[test]
    fn remove() {
        mut_vec!(v);
        v.push(1);
        assert_eq!(v.remove(0), 1);
    }
}
