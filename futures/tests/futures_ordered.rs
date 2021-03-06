#![feature(pin, arbitrary_self_types, futures_api)]

#[macro_use]
extern crate futures;

use futures::channel::oneshot;
use futures::executor::{block_on, block_on_stream};
use futures::future::{self, FutureObj};
use futures::prelude::*;
use futures::stream::{futures_ordered, FuturesOrdered};

mod support;

#[test]
fn works_1() {
    let (a_tx, a_rx) = oneshot::channel::<i32>();
    let (b_tx, b_rx) = oneshot::channel::<i32>();
    let (c_tx, c_rx) = oneshot::channel::<i32>();

    let mut stream = futures_ordered(vec![a_rx, b_rx, c_rx]);

    b_tx.send(99).unwrap();
    support::with_noop_waker_context(|cx| {
        assert!(stream.poll_next_unpin(cx).is_pending());
    });

    a_tx.send(33).unwrap();
    c_tx.send(33).unwrap();

    let mut iter = block_on_stream(stream);
    assert_eq!(Some(Ok(33)), iter.next());
    assert_eq!(Some(Ok(99)), iter.next());
    assert_eq!(Some(Ok(33)), iter.next());
    assert_eq!(None, iter.next());
}

#[test]
fn works_2() {
    let (a_tx, a_rx) = oneshot::channel::<i32>();
    let (b_tx, b_rx) = oneshot::channel::<i32>();
    let (c_tx, c_rx) = oneshot::channel::<i32>();

    let mut stream = futures_ordered(vec![
        FutureObj::new(Box::new(a_rx)),
        FutureObj::new(Box::new(b_rx.join(c_rx).map(|(a, b)| Ok(a? + b?)))),
    ]);

    support::with_noop_waker_context(|cx| {
        a_tx.send(33).unwrap();
        b_tx.send(33).unwrap();
        assert!(stream.poll_next_unpin(cx).is_ready());
        assert!(stream.poll_next_unpin(cx).is_pending());
        c_tx.send(33).unwrap();
        assert!(stream.poll_next_unpin(cx).is_ready());
    })
}

#[test]
fn from_iterator() {
    let stream = vec![
        future::ready::<i32>(1),
        future::ready::<i32>(2),
        future::ready::<i32>(3)
    ].into_iter().collect::<FuturesOrdered<_>>();
    assert_eq!(stream.len(), 3);
    assert_eq!(block_on(stream.collect::<Vec<_>>()), vec![1,2,3]);
}

/* ToDo: This requires FutureExt::select to be implemented
#[test]
fn queue_never_unblocked() {
    let (_a_tx, a_rx) = oneshot::channel::<Box<Any+Send>>();
    let (b_tx, b_rx) = oneshot::channel::<Box<Any+Send>>();
    let (c_tx, c_rx) = oneshot::channel::<Box<Any+Send>>();

    let mut stream = futures_ordered(vec![
        Box::new(a_rx) as Box<Future<Item = _, Error = _>>,
        Box::new(b_rx.select(c_rx).then(|res| Ok(Box::new(res) as Box<Any+Send>))) as _,
    ]);

    support::with_noop_waker_context(f)(|cx| {
        for _ in 0..10 {
            assert!(stream.poll_next(cx).unwrap().is_pending());
        }

        b_tx.send(Box::new(())).unwrap();
        assert!(stream.poll_next(cx).unwrap().is_pending());
        c_tx.send(Box::new(())).unwrap();
        assert!(stream.poll_next(cx).unwrap().is_pending());
        assert!(stream.poll_next(cx).unwrap().is_pending());
    })
}*/
