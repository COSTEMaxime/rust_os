use core::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
    sync::atomic::{AtomicU64,Ordering}
};
use alloc::boxed::Box;

pub mod simple_executor;
pub mod executor;
pub mod keyboard;

pub struct Task {
    id: TaskId,
    // dyn : dynamically dispatched, indicates that we store a trait object in the Box
    // Pin : prevent the value from being moved in memory (because futures might be self referential)
    future: Pin<Box<dyn Future<Output = ()>>>,
}

impl Task {
    pub fn new(future: impl Future<Output = ()> + 'static) -> Task {
        // create a new Task, move the future to the heap and pin it
        Task {
            id: TaskId::new(),
            future: Box::pin(future),
        }
    }

    fn poll(&mut self, context: &mut Context) -> Poll<()> {
        self.future.as_mut().poll(context)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct TaskId(u64);

impl TaskId {
    fn new() -> Self {
        static NEXT_ID: AtomicU64 = AtomicU64::new(0);
        // every ID is returned exactly once
        TaskId(NEXT_ID.fetch_add(1, Ordering::Relaxed))
    }
}