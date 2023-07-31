#![allow(unused)]
use std::future::Future;
use std::iter::Once;
use std::ops::{Deref, DerefMut};
use std::pin::Pin;
use std::task::{Context, Poll};

use pin_project_lite::pin_project;

pub trait LendingStream {
    type Item<'a>
    where
        Self: 'a;
    type NextFuture<'a>: Future<Output = Option<Self::Item<'a>>>
    where
        Self: 'a;

    fn next<'a>(&'a mut self) -> Self::NextFuture<'a>
    where
        Self: 'a,
        Self: Unpin;
}

impl<T: Unpin> LendingStream for Once<T> {
    type Item<'a> = T where T: 'a;
    type NextFuture<'a> = impl Future<Output = Option<Self::Item<'a>>> where T: 'a;

    fn next<'a>(&'a mut self) -> Self::NextFuture<'a>
    where
        Self: 'a,
        Self: Unpin,
    {
        async move {
            if let Some(item) = Iterator::next(self) {
                Some(item)
            } else {
                None
            }
        }
    }
}

impl<F, Fut, T: Unpin> LendingStream for F
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Option<T>>,
{
    type Item<'a> = T where Self: 'a;
    type NextFuture<'a> = impl Future<Output = Option<Self::Item<'a>>> where Self: 'a;

    fn next<'a>(&'a mut self) -> Self::NextFuture<'a>
    where
        Self: 'a,
        Self: Unpin,
    {
        (self)()
    }
}
