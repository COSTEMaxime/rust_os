use super::Task;
use alloc::collections::VecDeque;
use core::task::{Waker, RawWaker, RawWakerVTable, Context, Poll};

pub struct SimpleExecutor {
    task_queue: VecDeque<Task>,
}

impl SimpleExecutor {
    pub fn new() -> SimpleExecutor {
        SimpleExecutor {
            task_queue: VecDeque::new(),
        }
    }

    pub fn spawn(&mut self, task: Task) {
        self.task_queue.push_back(task)
    }
}

fn dummy_raw_waker() -> RawWaker {
    fn no_op(_: *const ()) {}
    fn clone(_: *const ()) -> RawWaker {
        dummy_raw_waker()
    }

    let vtable = &RawWakerVTable::new(clone, no_op, no_op, no_op);
    // data pointer is null because we don't use it in clone / no_op functions
    RawWaker::new(0 as *const (), vtable)
}

fn dummy_waker() -> Waker {
    unsafe { Waker::from_raw(dummy_raw_waker()) }
}

impl SimpleExecutor {
    pub fn run(&mut self) {
        // keep polling all queued tasks in a loop until all tasks are done
        // simple but we don't use notifications from the Waker
        while let Some(mut task) = self.task_queue.pop_front() {
            let waker = dummy_waker();
            let mut context = Context::from_waker(&waker);
            match task.poll(&mut context) {
                // task done
                Poll::Ready(()) => {}
                // add the task back to the queue
                Poll::Pending => self.task_queue.push_back(task),
            }
        }
    }
}