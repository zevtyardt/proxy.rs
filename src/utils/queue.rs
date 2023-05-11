use std::{
    collections::VecDeque,
    sync::{Arc, Condvar, Mutex},
};

use lazy_static::lazy_static;

lazy_static! {
    static ref CV: Condvar = Condvar::new();
}

#[derive(Debug, Clone)]
pub struct FifoQueue<T> {
    data: Arc<Mutex<VecDeque<T>>>,
}

impl<T: std::cmp::PartialEq> FifoQueue<T> {
    pub fn new() -> Self {
        Self {
            data: Arc::new(Mutex::new(VecDeque::new())),
        }
    }

    pub fn push(&self, value: T) {
        let mut data = self.data.lock().unwrap();
        data.push_back(value);
        CV.notify_one();
    }

    pub fn pop(&self) -> T {
        let mut data = self.data.lock().unwrap();
        while data.is_empty() {
            data = CV.wait(data).unwrap();
        }
        data.pop_front().unwrap()
    }

    pub fn qsize(&self) -> usize {
        let data = self.data.lock().unwrap();
        data.len()
    }

    pub fn is_empty(&self) -> bool {
        let data = self.data.lock().unwrap();
        data.is_empty()
    }

    pub fn is_unique(&self, value: T) -> bool {
        let data = self.data.lock().unwrap();
        !data.contains(&value)
    }
}

impl<T: std::cmp::PartialEq> std::fmt::Display for FifoQueue<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "<FifoQueue {} items>", self.qsize())
    }
}
