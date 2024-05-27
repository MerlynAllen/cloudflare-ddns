use std::{task, thread};
use std::cmp::{Ordering, Reverse};
use std::collections::BinaryHeap;
use std::rc::Rc;
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime};

use chrono::{Local, Utc};
use log::debug;

pub struct Scheduler {
    schedules: Vec<Schedule>,
    tasks: BinaryHeap<Task>,
}


pub type HandlerFn = dyn Fn() -> Option<()> + Send + Sync;

struct Schedule {
    interval: Duration,
    task_name: String,
    handler: Arc<HandlerFn>,
}


struct Task {
    name: String,
    at: Reverse<Instant>,
    created_at: (SystemTime, Instant),
    interval: Duration,
    worker: Arc<HandlerFn>,
}

impl Eq for Task {}

impl PartialEq<Self> for Task {
    fn eq(&self, other: &Self) -> bool {
        self.at.eq(&other.at)
    }
}

impl PartialOrd<Self> for Task {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.at.partial_cmp(&other.at)
    }
}

impl Ord for Task {
    fn cmp(&self, other: &Self) -> Ordering {
        self.at.cmp(&other.at)
    }
}


impl Scheduler {
    pub fn new() -> Self {
        Scheduler {
            schedules: Vec::new(),
            tasks: BinaryHeap::new(),
        }
    }
    pub fn schedule(&mut self, task_name: String, func: Arc<HandlerFn>, interval: Duration) {
        self.schedules.push(
            Schedule {
                task_name,
                interval,
                handler: func,
            }
        )
    }


    pub fn run(mut self) {
        // First, add all scheduled tasks to heap
        let now = Instant::now();
        let now_system_time = SystemTime::now();
        for schedule in self.schedules {
            self.tasks.push(
                Task {
                    name: schedule.task_name,
                    at: Reverse(now),
                    created_at: (now_system_time, now),
                    interval: schedule.interval,
                    worker: schedule.handler,
                }
            )
        }
        // Main loop
        // Pop and block until the time is up, fire it at once
        loop {
            let mut task = match self.tasks.pop() {
                Some(t) => t,
                None => continue,
            };
            debug!("Next task [{}] will execute at: {}", task.name, chrono::DateTime::<Local>::from(task.get_system_time()));
            thread::sleep_until(task.at.0);
            // Time's up
            debug!("Task [{}] time's up.", task.name);
            let task_handler_fn_ptr = task.worker.clone();
            let task_name_moved_in = task.name.clone();
            thread::spawn(move || {
                let result = task_handler_fn_ptr();
                debug!("Task [{}] exit {}.", task_name_moved_in,
                if result.is_some() { "normally" } else { "with error" })
            });
            task.at.0 += task.interval;
            debug!("Task [{}] rescheduled at {}", task.name, chrono::DateTime::<Local>::from(task.get_system_time()));
            self.tasks.push(task)
        }
    }
}

impl Task {
    fn get_system_time(&self) -> SystemTime {
        self.created_at.0 + (self.at.0 - self.created_at.1)
    }
}

#[test]
fn test_scheduler() {
    pretty_env_logger::init();
    let mut scheduler = Scheduler::new();
    let task1 = || {
        println!("1 every 5 secs");
        Some(())
    };
    let task2 = || {
        println!("2 every 3 secs");
        Some(())
    };
    let task3 = || {
        println!("3 every 2 secs");
        Some(())
    };
    scheduler.schedule(
        "Test 1".to_string(),
        Arc::new(task1),
        Duration::from_secs(5),
    );
    scheduler.schedule(
        "Test 2".to_string(),
        Arc::new(task2),
        Duration::from_secs(3),
    );
    scheduler.schedule(
        "Test 3".to_string(),
        Arc::new(task3),
        Duration::from_secs(2),
    );
    scheduler.run();
}