use super::{assert_is_word_aligned, PAGE_SIZE, unchecked_unwrap};
use const_init::ConstInit;
use core::alloc::{AllocErr, Opaque};
use core::cell::UnsafeCell;
use core::ptr::NonNull;
use memory_units::Pages;

extern "C" {
    #[link_name = "llvm.wasm.grow.memory.i32"]
    fn grow_memory(pages: usize) -> i32;
}

pub(crate) unsafe fn alloc_pages(n: Pages) -> Result<NonNull<Opaque>, AllocErr> {
    let ptr = grow_memory(n.0);
    if -1 != ptr {
        let ptr = (ptr as usize * PAGE_SIZE.0) as *mut Opaque;
        assert_is_word_aligned(ptr as *mut u8);
        Ok(unchecked_unwrap(NonNull::new(ptr)))
    } else {
        Err(AllocErr)
    }
}

pub(crate) struct Exclusive<T> {
    inner: UnsafeCell<T>,

    #[cfg(feature = "extra_assertions")]
    in_use: Cell<bool>,
}

impl<T: ConstInit> ConstInit for Exclusive<T> {
    const INIT: Self = Exclusive {
        inner: UnsafeCell::new(T::INIT),

        #[cfg(feature = "extra_assertions")]
        in_use: Cell::new(false),
    };
}

extra_only! {
    fn assert_not_in_use<T>(excl: &Exclusive<T>) {
        assert!(!excl.in_use, "`Exclusive<T>` is not re-entrant");
    }
}

extra_only! {
    fn set_in_use<T>(excl: &Exclusive<T>) {
        excl.in_use.set(true);
    }
}

extra_only! {
    fn set_not_in_use<T>(excl: &Exclusive<T>) {
        excl.in_use.set(false);
    }
}

impl<T> Exclusive<T> {
    /// Get exclusive, mutable access to the inner value.
    ///
    /// # Safety
    ///
    /// It is the callers' responsibility to ensure that `f` does not re-enter
    /// this method for this `Exclusive` instance.
    //
    // XXX: If we don't mark this function inline, then it won't be, and the
    // code size also blows up by about 200 bytes.
    #[inline]
    pub(crate) unsafe fn with_exclusive_access<'a, F, U>(&'a self, f: F) -> U
    where
        for<'x> F: FnOnce(&'x mut T) -> U,
    {
        assert_not_in_use(self);
        set_in_use(self);
        let result = f(&mut *self.inner.get());
        set_not_in_use(self);
        result
    }
}
