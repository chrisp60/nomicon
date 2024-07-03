use std::{
    cell::UnsafeCell,
    ops::{Deref, DerefMut},
};

/// A memory location that can be updated through a shared reference.
#[derive(Debug, Default)]
pub struct Cell<T> {
    value: UnsafeCell<T>,
}

impl<T> Cell<T> {
    /// Returns a new [`Cell<T>`] with the value set to `T`.
    pub const fn new(value: T) -> Self {
        Self {
            value: UnsafeCell::new(value),
        }
    }

    /// Replace the value in the [`Cell<T>`] with `value`.
    pub const fn set(&self, value: T)
    where
        T: Copy,
    {
        // SAFETY:
        // * self is !Sync, so no other thread can mutate this value.
        // * self never releases a shared or mutable reference.
        unsafe {
            *self.value.get().as_mut_unchecked() = value;
        }
    }

    /// Copy out the value from within the [`Cell<T>`].
    pub const fn get(&self) -> T
    where
        T: Copy,
    {
        // SAFETY:
        // * self is !Sync, so no other thread can mutate this value.
        // * self never releases a shared or mutable reference.
        unsafe { *self.value.get().as_ref_unchecked() }
    }
}

impl<T: Copy> Clone for Cell<T> {
    fn clone(&self) -> Self {
        Self::new(self.get())
    }
}

impl<T> From<T> for Cell<T> {
    fn from(value: T) -> Self {
        Self::new(value)
    }
}

#[derive(Debug, Default)]
pub struct RefCell<T> {
    value: UnsafeCell<T>,
    state: Cell<RefState>,
}

impl<T> RefCell<T> {
    pub const fn new(value: T) -> Self {
        Self {
            value: UnsafeCell::new(value),
            state: Cell::new(RefState::Unshared),
        }
    }

    /// Return a mutable handle to the value.
    ///
    /// # Panics
    /// If the value is currently borrowed.
    ///
    /// ```
    /// use nomicon::cell::RefCell;
    /// let r = RefCell::new("Hello".to_string());
    /// *r.borrow_mut() = "Foo".into();
    /// assert_eq!(r.borrow().as_str(), "Foo");
    /// ```
    pub fn borrow_mut(&self) -> RefMut<'_, T> {
        match self.try_borrow_mut() {
            Some(r) => r,
            None => panic!("Already borrowed"),
        }
    }

    /// Return a shared reference to the value.
    ///
    /// # Panics
    /// If the value is already borrowed through an exclusive reference.
    ///
    /// ```
    /// use nomicon::cell::RefCell;
    /// let r = RefCell::new([1, 2, 3]);
    /// {
    ///     let mut exclusive = r.borrow_mut();
    ///     exclusive[0] = 5;
    /// } // explicitly end the mut borrow
    ///
    /// let shared = r.borrow();
    /// assert_eq!(*shared, [5, 2, 3]);
    /// ```
    pub fn borrow(&self) -> Ref<'_, T> {
        match self.try_borrow() {
            Some(r) => r,
            None => panic!("Already exclusively borrowed"),
        }
    }

    /// Return a shared reference to the value if no exclusive references exist.
    ///
    /// ```
    /// use nomicon::cell::RefCell;
    ///
    /// let r = RefCell::new(Vec::new());
    ///
    /// let mut exclusive = r.borrow_mut();
    /// let shared = r.try_borrow();
    ///
    /// assert!(shared.is_none());
    ///
    /// exclusive.push(1);
    /// ```
    pub const fn try_borrow(&self) -> Option<Ref<'_, T>> {
        match self.state.get() {
            RefState::Unshared => {
                self.state.set(RefState::Shared(1));
                Some(Ref { refcell: self })
            }
            RefState::Exclusive => None,
            RefState::Shared(count) => {
                self.state.set(RefState::Shared(count + 1));
                Some(Ref { refcell: self })
            }
        }
    }

    /// Returns an exclusive reference to the value if it is not borrowed.
    ///
    /// ```
    /// use nomicon::cell::RefCell;
    ///
    /// let r = RefCell::new(Vec::<u8>::new());
    ///
    /// {
    ///     let shared = r.borrow();
    ///     let exclusive = r.try_borrow_mut();
    ///     assert!(exclusive.is_none());
    ///
    ///     assert_eq!(shared.len(), 0);
    /// }
    ///
    /// let mut new_exclusive = r.try_borrow_mut().unwrap();
    /// new_exclusive.push(5);
    /// ```
    pub const fn try_borrow_mut(&self) -> Option<RefMut<'_, T>> {
        match self.state.get() {
            RefState::Unshared => {
                self.state.set(RefState::Exclusive);
                Some(RefMut { refcell: self })
            }
            _ => None,
        }
    }
}

/// Allows a [`DerefMut`] implementation for `T`.
///
/// This type can be constructed through [`RefCell::try_borrow_mut`] and
/// [`RefCell::borrow_mut`].
pub struct RefMut<'a, T> {
    refcell: &'a RefCell<T>,
}

impl<'a, T> Drop for RefMut<'a, T> {
    fn drop(&mut self) {
        self.refcell.state.set(RefState::Unshared);
    }
}

impl<'a, T> Deref for RefMut<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe {
            // SAFETY:
            // * RefMut is only given out when there are no shared references and no
            //   exclusive references.
            // * self is !Send
            self.refcell.value.get().as_ref_unchecked()
        }
    }
}

impl<'a, T> DerefMut for RefMut<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe {
            // SAFETY:
            // * RefMut is only given out when there are no shared references and no
            //   exclusive references.
            // * self is !Send
            self.refcell.value.get().as_mut_unchecked()
        }
    }
}

pub struct Ref<'a, T> {
    refcell: &'a RefCell<T>,
}

impl<'a, T> Deref for Ref<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe {
            // SAFETY:
            // * Ref is only given out when there are no exclusive references.
            // * self in !Send
            self.refcell.value.get().as_ref_unchecked()
        }
    }
}

impl<'a, T> Drop for Ref<'a, T> {
    fn drop(&mut self) {
        match self.refcell.state.get() {
            RefState::Unshared | RefState::Exclusive => unreachable!(),
            RefState::Shared(1) => {
                self.refcell.state.set(RefState::Unshared);
            }
            RefState::Shared(count) => {
                let new = RefState::Shared(count - 1);
                self.refcell.state.set(new);
            }
        }
    }
}

#[derive(Debug, Copy, Clone, Default)]
enum RefState {
    #[default]
    Unshared,
    Exclusive,
    Shared(usize),
}
