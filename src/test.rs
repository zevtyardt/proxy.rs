use std::{net::IpAddr, thread};

use futures_util::{stream, StreamExt};

use crate::{
    resolver::{self, Resolver},
    utils::queue::FifoQueue,
    RUNTIME,
};

#[test]
fn resolver_get_real_ext_ip() {
    RUNTIME.block_on(async {
        let resolver = Resolver::new();
        let real_ext_ip = resolver.get_real_ext_ip().await;
        assert!(real_ext_ip.is_some());
    });
}

#[test]
fn resolver_resolve_host() {
    RUNTIME.block_on(async {
        let resolver = Resolver::new();
        for _ in 0..3 {
            let c_resolver = resolver.clone();
            tokio::task::spawn_blocking(move || {
                let ip = c_resolver.resolve("yahoo.com".to_string());
                assert!(ip.is_some());
                let ip = c_resolver.resolve("google.com".to_string());
                assert!(ip.is_some())
            })
            .await
            .unwrap();
        }

        let cached = resolver::CACHED_HOSTS.lock();
        assert!(cached.is_ok());
        assert_eq!(cached.unwrap().len(), 2);
    })
}
#[test]
fn resolver_get_ip_info() {
    RUNTIME.block_on(async {
        let resolver = Resolver::new();
        let real_ext_ip = resolver.get_real_ext_ip().await;
        assert!(real_ext_ip.is_some());

        let real_ext_ip = real_ext_ip.unwrap().parse::<IpAddr>();
        assert!(real_ext_ip.is_ok());

        let geodata = resolver.get_ip_info(real_ext_ip.unwrap()).await;

        assert_ne!(geodata.name, "unknown");
    });
}

#[test]
fn queue_basic() {
    let queue = FifoQueue::new();

    // Test push and pop
    queue.push(1);
    queue.push(2);
    queue.push(3);

    assert_eq!(queue.pop(), 1);
    assert_eq!(queue.pop(), 2);
    assert_eq!(queue.pop(), 3);

    // Test size and is_empty
    assert_eq!(queue.qsize(), 0);
    assert!(queue.is_empty());

    queue.push(4);
    queue.push(5);

    assert_eq!(queue.qsize(), 2);
    assert!(!queue.is_empty());
}

#[test]
fn queue_thread_safety() {
    // createa a queue of numbers
    let queue = FifoQueue::<i32>::new();

    let q1 = queue.clone();
    let t1 = thread::spawn(move || {
        q1.push(1);
        q1.push(2);
    });

    let q2 = queue.clone();
    let t2 = thread::spawn(move || {
        q2.push(3);
        q2.push(4)
    });

    t1.join().unwrap();
    t2.join().unwrap();

    assert_eq!(queue.qsize(), 4);
}

#[test]
fn queue_parallel_async() {
    RUNTIME.block_on(async {
        let queue = FifoQueue::new();
        let mut fut = vec![];

        let q1 = queue.clone();
        fut.push(tokio::spawn(async move {
            q1.push(1);
            q1.push(2);
        }));

        let q2 = queue.clone();
        fut.push(tokio::spawn(async move {
            q2.push(3);
            q2.push(4);
        }));

        stream::iter(fut)
            .buffer_unordered(10)
            .collect::<Vec<_>>()
            .await;

        assert_eq!(queue.qsize(), 4);
    })
}
