use std::{
    cell::UnsafeCell,
    mem::ManuallyDrop,
    ops::Deref,
    process::abort,
    ptr::NonNull,
    sync::atomic::{
        fence, AtomicUsize,
        Ordering::{Acquire, Relaxed, Release},
    },
};

struct Data<T> {
    // number of Arc
    arc_count: AtomicUsize,
    // number of Weak, plus 1 for representing all of Arcs
    alloc_count: AtomicUsize,

    data: UnsafeCell<ManuallyDrop<T>>,
}

pub struct Weak<T> {
    ptr: NonNull<Data<T>>,
}

impl<T> Weak<T> {
    pub fn upgrade(&self) -> Arc<T> {
        todo!()
    }

    fn data(&self) -> &Data<T> {
        unsafe { self.ptr.as_ref() }
    }
}

unsafe impl<T: Sync + Send> Send for Weak<T> {}
unsafe impl<T: Sync + Send> Sync for Weak<T> {}

impl<T> Drop for Weak<T> {
    fn drop(&mut self) {
        if self.data().alloc_count.fetch_sub(1, Release) == 1 {
            fence(Acquire);
            unsafe { drop(Box::from_raw(self.ptr.as_ptr())) }
        }
    }
}

pub struct Arc<T> {
    ptr: NonNull<Data<T>>,
}

impl<T> Arc<T> {
    pub fn new(data: T) -> Self {
        let data = Box::new(Data {
            arc_count: AtomicUsize::new(1),
            alloc_count: AtomicUsize::new(1),
            data: UnsafeCell::new(ManuallyDrop::new(data)),
        });
        Self {
            ptr: NonNull::from(Box::leak(data)),
        }
    }

    pub fn downgrade(&self) -> Weak<T> {
        todo!()
    }

    fn data(&self) -> &Data<T> {
        unsafe { self.ptr.as_ref() }
    }
}

unsafe impl<T: Sync + Send> Send for Arc<T> {}
unsafe impl<T: Sync + Send> Sync for Arc<T> {}

impl<T> Deref for Arc<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.data().data.get() }
    }
}

impl<T> Clone for Arc<T> {
    fn clone(&self) -> Self {
        if self.data().arc_count.fetch_add(1, Relaxed) > usize::MAX / 2 {
            abort();
        }
        Self { ptr: self.ptr }
    }
}

impl<T> Drop for Arc<T> {
    fn drop(&mut self) {
        if self.data().arc_count.fetch_sub(1, Release) == 1 {
            fence(Acquire);
            unsafe { ManuallyDrop::drop(&mut *self.data().data.get()) }
        }
    }
}

#[test]
fn test() {
    static NUM_DROPS: AtomicUsize = AtomicUsize::new(0);

    struct DetectDrop;

    impl Drop for DetectDrop {
        fn drop(&mut self) {
            NUM_DROPS.fetch_add(1, Relaxed);
        }
    }

    let x = Arc::new(("hello", DetectDrop));
    let y = x.clone();
    let z = x.clone();

    let t = std::thread::spawn({
        let x = x.clone();
        move || {
            assert_eq!(x.0, "hello");
        }
    });
    assert_eq!(x.0, "hello");
    t.join().unwrap();

    assert_eq!(NUM_DROPS.load(Relaxed), 0);

    drop(x);
    drop(y);
    drop(z);

    assert_eq!(NUM_DROPS.load(Relaxed), 1);
}
