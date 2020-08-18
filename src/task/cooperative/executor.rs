use super::super::{AsyncTask, Tid};
use alloc::{sync::Arc, collections::BTreeMap, task::Wake};
use core::task::{Waker, Context, Poll};
use crossbeam_queue::ArrayQueue;

pub struct Executor {
tasks: BTreeMap<Tid, AsyncTask>,
task_queue: Arc<ArrayQueue<Tid>>,
waker_cache: BTreeMap<Tid, Waker>
}

struct TaskWaker {
    task_id: Tid,
    task_queue: Arc<ArrayQueue<Tid>>,
}

impl Executor {
pub fn new() -> Self {
Executor {
tasks: BTreeMap::new(),
task_queue: Arc::new(ArrayQueue::new(1024)),
waker_cache: BTreeMap::new()
}
}

pub fn spawn(&mut self, task: AsyncTask) {
let id = task.id;
if self.tasks.insert(task.id, task).is_some() {
panic!("Task {:?} is already in the queue!", id);
}
self.task_queue.push(id).unwrap_or_else(|e| panic!("Task queue full: {}!", e));
}

fn execute_ready(&mut self) {
let Self {tasks, task_queue, waker_cache} = self;
while let Ok(task_id) = task_queue.pop() {
let task = match tasks.get_mut(&task_id) {
Some(task) => task,
None => continue,
};
let waker = waker_cache.entry(task_id).or_insert_with(|| TaskWaker::new(task_id, task_queue.clone()));
let mut context = Context::from_waker(waker);
match task.poll(&mut context) {
Poll::Ready(()) => {
let _ = tasks.remove(&task_id);
let _ = waker_cache.remove(&task_id);
},
Poll::Pending => {}
}
}
}

    pub fn run(&mut self) -> ! {
    loop {
self.execute_ready();
self.sleep_if_idle();
}
}

    fn sleep_if_idle(&self) {
        use x86_64::instructions::interrupts::{self, enable_interrupts_and_hlt};
        interrupts::disable();
        if self.task_queue.is_empty() {
            enable_interrupts_and_hlt();
        } else {
            interrupts::enable();
        }
    }
}

impl TaskWaker {
    fn new(task_id: Tid, task_queue: Arc<ArrayQueue<Tid>>) -> Waker {
        Waker::from(Arc::new(TaskWaker {
            task_id,
            task_queue,
        }))
}

    fn wake_task(&self) {
        self.task_queue.push(self.task_id).expect("task_queue full");
    }
}

impl Wake for TaskWaker {
    fn wake(self: Arc<Self>) {
        self.wake_task();
    }

    fn wake_by_ref(self: &Arc<Self>) {
        self.wake_task();
    }
}
