use conquer_once::spin::OnceCell;
use crossbeam_queue::ArrayQueue;
use futures_util::{
    StreamExt,
    stream::Stream,
    task::AtomicWaker
};
use core::{
    pin::Pin,
    task::{
        Poll, Context
    }
};
use pc_keyboard::{Keyboard, ScancodeSet1, layouts, HandleControl, DecodedKey};

use crate::{print, println};

// use OneCell to perform safe one-time initialization of static
// advanatge over lazy_static : initialization does not happen in the interrupt handler
static SCANCODE_QUEUE: OnceCell<ArrayQueue<u8>> = OnceCell::uninit();

static WAKER: AtomicWaker = AtomicWaker::new();

// pub(crate) so we can use it only inside lib.rs
pub(crate) fn add_scancode(scancode: u8) {
    // try to get a reference to the scancode queue
    if let Ok(queue) = SCANCODE_QUEUE.try_get() {
        if let Err(_) = queue.push(scancode) {
            println!("WARNING: scancode queue full, dropping keyboard input");
        } else {
            // if a waker is registered in the WAKER, wake() will notify the registered waker executor
            WAKER.wake();
        }
    } else {
        println!("WARNING: scnacode queue not initialized");
    }
}


// private field prevent construction from outside of the module
pub struct ScancodeStream {
    _private: (),
}

impl ScancodeStream {
    pub fn new() -> Self {
        // panic if queue is already initialized
        SCANCODE_QUEUE.try_init_once(|| ArrayQueue::new(100))
            .expect("ScancodeStream::new should only be called once");
        ScancodeStream { _private: () }
    }
}

impl Stream for ScancodeStream {
    type Item = u8;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<u8>> {
        let queue = SCANCODE_QUEUE.try_get().expect("not initialized");
        
        // avoid performance oberhead of registering  a waker if the queue is not empty
        if let Ok(scancode) = queue.pop() {
            return Poll::Ready(Some(scancode));
        }
        
        WAKER.register(&cx.waker());
        match queue.pop() {
            Ok(scancode) => {
                // remove waker
                WAKER.take();
                Poll::Ready(Some(scancode))
            }
            // queue is empty, but with a registered waker
            Err(crossbeam_queue::PopError) => Poll::Pending,
        }
    }
}

pub async fn print_keypresses() {
    let mut scancodes = ScancodeStream::new();
    let mut keyboard = Keyboard::new(layouts::ANSI103fr, ScancodeSet1, HandleControl::Ignore);

    while let Some(scancode) = scancodes.next().await {
        if let Ok(Some(key_event)) = keyboard.add_byte(scancode) {
            if let Some(key) = keyboard.process_keyevent(key_event) {
                match key {
                    DecodedKey::Unicode(character) => print!("{}", character),
                    DecodedKey::RawKey(key) => print!("{:?}", key),
                }
            }
        }
    }
}