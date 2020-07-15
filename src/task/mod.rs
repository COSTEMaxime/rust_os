use core::{future::Future, pin::Pin};
use core::task::{Context, Poll};
use alloc::boxed::Box;

pub mod simple_executor;
// pub mod keyboard;

pub struct Task {
    // dyn : dynamically dispatched, indicates that we store a trait object in the Box
    // Pin : prevent the value from being moved in memory (because futures might be self referential)
    future: Pin<Box<dyn Future<Output = ()>>>,
}

impl Task {
    pub fn new(future: impl Future<Output = ()> + 'static) -> Task {
        // create a new Task, move the future to the heap and pin it
        Task {
            future: Box::pin(future),
        }
    }

    fn poll(&mut self, context: &mut Context) -> Poll<()> {
        self.future.as_mut().poll(context)
    }
}