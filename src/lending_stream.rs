#![allow(unused)]
use std::future::Future;
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
