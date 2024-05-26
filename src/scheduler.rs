use std::cmp::{Ordering, Reverse};
use std::collections::BinaryHeap;
use std::time::{Duration, Instant};
use std::thread;

pub struct Scheduler {
    schedules: Vec<Schedule>,
    tasks: BinaryHeap<Task>,
}


pub type HandlerFn = fn() -> Option<()>;

struct Schedule {
    interval: Duration,
    handler: Box<HandlerFn>,
}


#[derive(Debug)]
struct Task {
    at: Reverse<Instant>,
    interval: Duration,
    worker: Box<HandlerFn>,
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
    pub fn schedule(&mut self, func: Box<HandlerFn>, interval: Duration) {
        self.schedules.push(
            Schedule {
                interval,
                handler: func,
            }
        )
    }

    pub fn run(mut self) {
        // First, add all scheduled tasks to heap
        let now = Instant::now();
        for schedule in self.schedules {
            self.tasks.push(
                Task {
                    at: Reverse(now),
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
            thread::sleep_until(task.at.0);
            // Time's up
            thread::spawn(task.worker.clone());
            task.at.0 += task.interval;
            self.tasks.push(task)
        }
    }
}

#[test]
fn test_scheduler() {
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
        Box::new(task1),
        Duration::from_secs(5),
    );
    scheduler.schedule(
        Box::new(task2),
        Duration::from_secs(3),
    );
    scheduler.schedule(
        Box::new(task3),
        Duration::from_secs(2),
    );
    scheduler.run();
}