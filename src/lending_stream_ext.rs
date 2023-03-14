use std::future::Future;

use pin_project_lite::pin_project;

use crate::lending_stream::LendingStream;

pub trait LendingStreamExt: LendingStream {
    async fn try_next<'a, T, E>(&'a mut self) -> Result<Option<T>, E>
    where
        Self: LendingStream<Item<'a> = Result<T, E>> + Unpin,
        Self: 'a,
    {
        self.next().await.transpose()
    }

    async fn count(mut self) -> usize
    where
        Self: Sized + Unpin,
    {
        let mut count = 0;
        while self.next().await.is_some() {
            count += 1;
        }
        count
    }

    fn map<T, F>(self, f: F) -> Map<Self, F>
    where
        Self: Sized,
        F: FnMut(Self::Item<'_>) -> T,
    {
        Map { stream: self, f }
    }

    fn flat_map<U, F>(self, f: F) -> FlatMap<Self, U, F>
    where
        Self: Sized,
        U: LendingStream,
        F: FnMut(Self::Item<'_>) -> U,
    {
        FlatMap {
            stream: self.map(f),
            inner_stream: None,
        }
    }

    fn flatten<'a>(self) -> Flatten<Self, Option<Self::Item<'a>>>
    where
        Self: Sized,
        Self::Item<'a>: LendingStream,
    {
        Flatten {
            stream: self,
            inner_stream: None,
        }
    }

    fn then<F, Fut>(self, f: F) -> Then<Self, F, Fut>
    where
        Self: Sized,
        F: FnMut(Self::Item<'_>) -> Fut,
        Fut: Future,
    {
        Then {
            stream: self,
            f,
            _pd: Default::default(),
        }
    }

    #[cfg(feature = "polonius")]
    fn filter<P>(self, predicate: P) -> Filter<Self, P>
    where
        Self: Sized,
        for<'a> P: FnMut(&Self::Item<'a>) -> bool,
    {
        Filter {
            stream: self,
            predicate,
        }
    }

    #[cfg(feature = "polonius")]
    fn filter_map<T, F>(self, f: F) -> FilterMap<Self, F>
    where
        Self: Sized,
        F: FnMut(Self::Item<'_>) -> Option<T>,
    {
        FilterMap { stream: self, f }
    }

    fn take(self, n: usize) -> Take<Self>
    where
        Self: Sized,
    {
        Take { stream: self, n }
    }

    fn take_while<P>(self, predicate: P) -> TakeWhile<Self, P>
    where
        Self: Sized,
        P: FnMut(&Self::Item<'_>) -> bool,
    {
        TakeWhile {
            stream: self,
            predicate,
        }
    }

    fn skip(self, n: usize) -> Skip<Self>
    where
        Self: Sized,
    {
        Skip { stream: self, n }
    }

    #[cfg(feature = "polonius")]
    fn skip_while<P>(self, predicate: P) -> SkipWhile<Self, P>
    where
        Self: Sized,
        P: FnMut(&Self::Item<'_>) -> bool,
    {
        SkipWhile {
            stream: self,
            predicate,
        }
    }

    fn step_by(self, step: usize) -> StepBy<Self>
    where
        Self: Sized,
    {
        assert!(step > 0, "`step` must be greater than zero");
        StepBy {
            stream: self,
            step,
            n: 0,
        }
    }

    fn chain<U>(self, other: U) -> Chain<Self, U>
    where
        Self: Sized,
        for<'a> U: LendingStream<Item<'a> = Self::Item<'a>> + Sized,
    {
        Chain {
            first: self.fuse(),
            second: other.fuse(),
        }
    }

    fn cloned<'a, T>(self) -> Cloned<Self>
    where
        Self: LendingStream<Item<'a> = &'a T> + Sized,
        T: Clone + 'a,
        Self: 'a,
    {
        Cloned { stream: self }
    }

    fn copied<'a, T>(self) -> Copied<Self>
    where
        Self: LendingStream<Item<'a> = &'a T> + Sized,
        T: Copy + 'a,
        Self: 'a,
    {
        Copied { stream: self }
    }

    async fn collect<C>(mut self) -> C
    where
        Self: Sized + Unpin,
        for<'a> C: Default + Extend<Self::Item<'a>>,
    {
        let mut items: C = Default::default();
        while let Some(item) = self.next().await {
            items.extend(Some(item));
        }
        items
    }

    async fn try_collect<C, T, E>(mut self) -> Result<C, E>
    where
        for<'a> Self: LendingStream<Item<'a> = Result<T, E>> + Sized + Unpin + 'a,
        C: Default + Extend<T>,
    {
        let mut items: C = Default::default();
        loop {
            match self.next().await {
                Some(Ok(item)) => items.extend(Some(item)),
                Some(Err(e)) => return Err(e),
                None => return Ok(items),
            }
        }
    }

    async fn partition<B, P>(mut self, mut predicate: P) -> (B, B)
    where
        Self: Sized + Unpin,
        for<'a> B: Default + Extend<Self::Item<'a>>,
        for<'a> P: FnMut(&Self::Item<'a>) -> bool,
    {
        let mut left: B = Default::default();
        let mut right: B = Default::default();
        while let Some(item) = self.next().await {
            if predicate(&item) {
                left.extend(Some(item));
            } else {
                right.extend(Some(item));
            }
        }
        (left, right)
    }

    async fn fold<T, F>(mut self, init: T, mut f: F) -> T
    where
        Self: Sized + Unpin,
        for<'a> F: FnMut(T, Self::Item<'a>) -> T,
    {
        let mut accum = init;
        while let Some(item) = self.next().await {
            accum = f(accum, item);
        }
        accum
    }

    async fn try_fold<T, E, F, B>(&mut self, init: B, mut f: F) -> Result<B, E>
    where
        for<'a> Self: LendingStream<Item<'a> = Result<T, E>> + Sized + Unpin + 'a,
        F: FnMut(B, T) -> Result<B, E>,
    {
        let mut accum = init;
        while let Some(item) = self.next().await {
            accum = f(accum, item?)?;
        }
        Ok(accum)
    }

    fn scan<St, B, F>(self, initial_state: St, f: F) -> Scan<Self, St, F>
    where
        Self: Sized,
        for<'a> F: FnMut(&mut St, Self::Item<'a>) -> Option<B>,
    {
        Scan {
            stream: self,
            state_f: (initial_state, f),
        }
    }

    fn fuse(self) -> Fuse<Self>
    where
        Self: Sized,
    {
        Fuse {
            stream: self,
            done: false,
        }
    }

    #[cfg(feature = "polonius")]
    fn cycle(self) -> Cycle<Self>
    where
        Self: Clone + Sized,
    {
        Cycle {
            orig: self.clone(),
            stream: self,
        }
    }

    fn enumerate(self) -> Enumerate<Self>
    where
        Self: Sized,
    {
        Enumerate { stream: self, i: 0 }
    }

    fn inspect<F>(self, f: F) -> Inspect<Self, F>
    where
        Self: Sized + Unpin,
        for<'a> F: FnMut(&Self::Item<'a>),
    {
        Inspect { stream: self, f }
    }

    async fn nth<T>(&mut self, mut n: usize) -> Option<T>
    where
        for<'a> Self: LendingStream<Item<'a> = T> + Sized + Unpin + 'a,
    {
        loop {
            let next = self.next().await;
            match next {
                Some(x) => {
                    if n == 0 {
                        return Some(x);
                    }
                    n -= 1;
                }
                None => return None,
            }
        }
    }

    async fn last<T>(mut self) -> Option<T>
    where
        for<'a> Self: LendingStream<Item<'a> = T> + Sized + Unpin + 'a,
    {
        let mut last = None;
        while let Some(x) = self.next().await {
            last = Some(x);
        }
        last
    }

    async fn find<P, T>(&mut self, mut predicate: P) -> Option<T>
    where
        Self: Sized + Unpin,
        P: FnMut(&Self::Item<'_>) -> bool,
        for<'a> Self: LendingStream<Item<'a> = T> + Sized + Unpin + 'a,
    {
        loop {
            let next = self.next().await;
            match next {
                Some(x) => {
                    if predicate(&x) {
                        return Some(x);
                    }
                }
                None => return None,
            }
        }
    }

    async fn find_map<F, B>(&mut self, mut f: F) -> Option<B>
    where
        Self: Sized + Unpin,
        F: FnMut(Self::Item<'_>) -> Option<B>,
    {
        while let Some(x) = self.next().await {
            if let Some(y) = f(x) {
                return Some(y);
            }
        }
        None
    }

    async fn position<P>(&mut self, mut predicate: P) -> Option<usize>
    where
        Self: LendingStream + Sized + Unpin,
        P: FnMut(Self::Item<'_>) -> bool,
    {
        let mut i = 0;
        self.find_map(|x| {
            let r = if predicate(x) { Some(i) } else { None };
            i += 1;
            r
        })
        .await
    }

    async fn all<P>(&mut self, mut predicate: P) -> bool
    where
        Self: LendingStream + Sized + Unpin,
        P: FnMut(Self::Item<'_>) -> bool,
    {
        loop {
            let next = self.next().await;
            match next {
                Some(x) => {
                    if !predicate(x) {
                        return false;
                    }
                }
                None => return true,
            }
        }
    }

    async fn any<P>(&mut self, mut predicate: P) -> bool
    where
        Self: LendingStream + Sized + Unpin,
        P: FnMut(Self::Item<'_>) -> bool,
    {
        loop {
            let next = self.next().await;
            match next {
                Some(x) => {
                    if predicate(x) {
                        return true;
                    }
                }
                None => return false,
            }
        }
    }

    async fn for_each<F>(mut self, mut f: F)
    where
        Self: Sized + Unpin,
        F: FnMut(Self::Item<'_>),
    {
        while let Some(x) = self.next().await {
            f(x);
        }
    }

    async fn try_for_each<F, E>(&mut self, mut f: F) -> Result<(), E>
    where
        Self: LendingStream + Sized + Unpin,
        F: FnMut(Self::Item<'_>) -> Result<(), E>,
    {
        while let Some(item) = self.next().await {
            f(item)?;
        }
        Ok(())
    }

    fn zip<U>(self, other: U) -> Zip<Self, U>
    where
        Self: Sized,
        U: LendingStream,
    {
        Zip {
            first: self,
            second: other,
        }
    }

    async fn unzip<A, B, FromA, FromB>(mut self) -> (FromA, FromB)
    where
        FromA: Default + Extend<A>,
        FromB: Default + Extend<B>,
        for<'a> Self: LendingStream<Item<'a> = (A, B)> + Sized + Unpin + 'a,
    {
        let mut res: (FromA, FromB) = (Default::default(), Default::default());
        while let Some((a, b)) = self.next().await {
            res.0.extend(Some(a));
            res.1.extend(Some(b));
        }
        res
    }

    fn or<S>(self, other: S) -> Or<Self, S>
    where
        Self: Sized,
        for<'a> S: LendingStream<Item<'a> = Self::Item<'a>>,
    {
        Or {
            stream1: self,
            stream2: other,
        }
    }

    fn race<S>(self, other: S) -> Race<Self, S>
    where
        Self: Sized,
        for<'a> S: LendingStream<Item<'a> = Self::Item<'a>>,
    {
        Race {
            stream1: self,
            stream2: other,
        }
    }
}

impl<T> LendingStreamExt for T where T: LendingStream {}

pin_project! {
    #[derive(Clone, Debug)]
    #[must_use = "streams do nothing unless polled"]
    pub struct Map<S, F> {
        #[pin]
        stream: S,
        f: F,
    }
}

impl<S: LendingStream + Unpin, F, T> LendingStream for Map<S, F>
where
    for<'a> F: FnMut(S::Item<'a>) -> T,
{
    type Item<'a> = T where F: 'a, S: 'a;
    type NextFuture<'a> = impl Future<Output=Option<Self::Item<'a>>> where F: 'a, S: 'a;

    fn next<'a>(&'a mut self) -> Self::NextFuture<'a>
    where
        Self: 'a,
        Self: Unpin,
    {
        async {
            let next = self.stream.next().await?;
            Some((self.f)(next))
        }
    }
}

macro flat_helper($self:ident) {
    async {
        loop {
            if $self.inner_stream.is_some() {
                let next = $self.inner_stream.as_mut().unwrap().next().await?;
                return Some(next);
            }

            let inner_stream = $self.stream.next().await?;
            let _ = $self.inner_stream.insert(inner_stream);
        }
    }
}

pin_project! {
    #[derive(Clone, Debug)]
    #[must_use = "streams do nothing unless polled"]
    pub struct FlatMap<S, U, F> {
        #[pin]
        stream: Map<S, F>,
        #[pin]
        inner_stream: Option<U>,
    }
}

impl<S, U, F> LendingStream for FlatMap<S, U, F>
where
    S: LendingStream + Unpin,
    U: LendingStream + Unpin,
    for<'a> F: FnMut(S::Item<'a>) -> U,
{
    type Item<'a> = U::Item<'a> where F: 'a, S: 'a, U: 'a;
    type NextFuture<'a> = impl Future<Output=Option<U::Item<'a>>> where F: 'a, S: 'a, U: 'a;

    fn next<'a>(&'a mut self) -> Self::NextFuture<'a>
    where
        Self: 'a,
        Self: Unpin,
    {
        flat_helper!(self)
    }
}

pin_project! {
    #[derive(Clone, Debug)]
    #[must_use = "streams do nothing unless polled"]
    pub struct Flatten<S: LendingStream, U> {
        #[pin]
        stream: S,
        #[pin]
        inner_stream: Option<U>,
    }
}

impl<S, U> LendingStream for Flatten<S, U>
where
    U: LendingStream + Unpin,
    for<'a> S::Item<'a>: LendingStream + Unpin,
    for<'a> S: LendingStream<Item<'a> = U> + Unpin + 'a,
{
    type Item<'a> = <<S as LendingStream>::Item<'a> as LendingStream>::Item<'a> where S: 'a, S::Item<'a>: 'a, Self: 'a;
    type NextFuture<'a> = impl Future<Output=Option<Self::Item<'a>>> where S: 'a, S::Item<'a>: 'a, Self: 'a;

    fn next<'a>(&'a mut self) -> Self::NextFuture<'a>
    where
        Self: 'a,
        Self: Unpin,
    {
        flat_helper!(self)
    }
}

pin_project! {
    #[derive(Clone, Debug)]
    #[must_use = "streams do nothing unless polled"]
    pub struct Then<S, F, Fut> {
        #[pin]
        stream: S,
        f: F,
        _pd: std::marker::PhantomData<fn() -> Fut>,
    }
}

impl<S, F, Fut> LendingStream for Then<S, F, Fut>
where
    S: LendingStream + Unpin,
    for<'a> F: FnMut(S::Item<'a>) -> Fut,
    Fut: Future,
{
    type Item<'a> = Fut::Output where F: 'a, S: 'a, Fut: 'a;
    type NextFuture<'a> = impl Future<Output=Option<Self::Item<'a>>> where F: 'a, S: 'a, Fut: 'a;

    fn next<'a>(&'a mut self) -> Self::NextFuture<'a>
    where
        Self: 'a,
        Self: Unpin,
    {
        async {
            let next = self.stream.next().await?;
            Some((self.f)(next).await)
        }
    }
}

#[cfg(feature = "polonius")]
pin_project! {
    #[derive(Clone, Debug)]
    #[must_use = "streams do nothing unless polled"]
    pub struct Filter<S, P> {
        #[pin]
        stream: S,
        predicate: P,
    }
}

#[cfg(feature = "polonius")]
impl<S, P> LendingStream for Filter<S, P>
where
    S: LendingStream + Unpin,
    for<'a> S::NextFuture<'a>: Future,
    for<'a> P: FnMut(&S::Item<'a>) -> bool,
{
    type Item<'a> = S::Item<'a> where S: 'a, P: 'a;
    type NextFuture<'a> = impl Future<Output=Option<Self::Item<'a>>> where S: 'a, P: 'a;

    fn next<'a>(&'a mut self) -> Self::NextFuture<'a>
    where
        Self: 'a,
        Self: Unpin,
    {
        async {
            loop {
                let next = self.stream.next().await?;
                if (self.predicate)(&next) {
                    return Some(next);
                }
            }
        }
    }
}

#[cfg(feature = "polonius")]
pin_project! {
    #[derive(Clone, Debug)]
    #[must_use = "streams do nothing unless polled"]
    pub struct FilterMap<S, F> {
        #[pin]
        stream: S,
        f: F,
    }
}

#[cfg(feature = "polonius")]
impl<S, F, T> LendingStream for FilterMap<S, F>
where
    S: LendingStream + Unpin,
    for<'a> S::NextFuture<'a>: Future,
    for<'a> F: FnMut(&S::Item<'a>) -> Option<T>,
{
    type Item<'a> = T where S: 'a, F: 'a;
    type NextFuture<'a> = impl Future<Output = Option<Self::Item<'a>>> where S:'a, F: 'a;

    fn next<'a>(&'a mut self) -> Self::NextFuture<'a>
    where
        Self: 'a,
        Self: Unpin,
    {
        async {
            loop {
                let next = self.stream.next().await;
                match next {
                    Some(next) => {
                        if let Some(next) = (self.f)(&next) {
                            return Some(next);
                        }
                    }
                    None => continue,
                }
            }
        }
    }
}

pin_project! {
    #[derive(Clone, Debug)]
    #[must_use = "streams do nothing unless polled"]
    pub struct Take<S> {
        #[pin]
        stream: S,
        n: usize,
    }
}

impl<S> LendingStream for Take<S>
where
    S: LendingStream + Unpin,
{
    type Item<'a> = S::Item<'a> where S: 'a;
    type NextFuture<'a> = impl Future<Output=Option<Self::Item<'a>>> where S: 'a;

    fn next<'a>(&'a mut self) -> Self::NextFuture<'a>
    where
        Self: 'a,
        Self: Unpin,
    {
        async {
            if self.n == 0 {
                None
            } else {
                let next = self.stream.next().await;
                match next {
                    Some(_) => self.n -= 1,
                    None => self.n = 0,
                }
                next
            }
        }
    }
}

pin_project! {
    #[derive(Clone, Debug)]
    #[must_use = "streams do nothing unless polled"]
    pub struct TakeWhile<S, P> {
        #[pin]
        stream: S,
        predicate: P,
    }
}

impl<S, P> LendingStream for TakeWhile<S, P>
where
    S: LendingStream + Unpin,
    for<'a> P: FnMut(&S::Item<'a>) -> bool,
{
    type Item<'a> = S::Item<'a> where S: 'a, P: 'a;
    type NextFuture<'a> = impl Future<Output = Option<Self::Item<'a>>> where S: 'a, P: 'a;

    fn next<'a>(&'a mut self) -> Self::NextFuture<'a>
    where
        Self: 'a,
        Self: Unpin,
    {
        async {
            let next = self.stream.next().await?;
            if (self.predicate)(&next) {
                Some(next)
            } else {
                None
            }
        }
    }
}

pin_project! {
    #[derive(Clone, Debug)]
    #[must_use = "streams do nothing unless polled"]
    pub struct Skip<S> {
        #[pin]
        stream: S,
        n: usize,
    }
}

impl<S> LendingStream for Skip<S>
where
    S: LendingStream + Unpin,
{
    type Item<'a> = S::Item<'a> where S: 'a;
    type NextFuture<'a> = impl Future<Output=Option<Self::Item<'a>>> where S: 'a;

    fn next<'a>(&'a mut self) -> Self::NextFuture<'a>
    where
        Self: 'a,
        Self: Unpin,
    {
        async {
            while self.n > 0 {
                let _ = self.stream.next().await?;
                self.n -= 1;
            }
            self.stream.next().await
        }
    }
}

#[cfg(feature = "polonius")]
pin_project! {
    #[derive(Clone, Debug)]
    #[must_use = "streams do nothing unless polled"]
    pub struct SkipWhile<S, P> {
        #[pin]
        stream: S,
        predicate: P,
    }
}

#[cfg(feature = "polonius")]
impl<S, P> LendingStream for SkipWhile<S, P>
where
    S: LendingStream + Unpin,
    for<'a> P: FnMut(&S::Item<'a>) -> bool,
{
    type Item<'a> = S::Item<'a> where S: 'a, P: 'a;
    type NextFuture<'a> = impl Future<Output = Option<Self::Item<'a>>> where S: 'a, P: 'a;

    fn next<'a>(&'a mut self) -> Self::NextFuture<'a>
    where
        Self: 'a,
        Self: Unpin,
    {
        async {
            loop {
                let next = self.stream.next().await?;
                if !(self.predicate)(&next) {
                    return Some(next);
                }
            }
        }
    }
}

pin_project! {
    #[derive(Clone, Debug)]
    #[must_use = "streams do nothing unless polled"]
    pub struct StepBy<S> {
        #[pin]
        stream: S,
        step: usize,
        n: usize,
    }
}

impl<S> LendingStream for StepBy<S>
where
    S: LendingStream + Unpin,
{
    type Item<'a> = S::Item<'a> where S: 'a;
    type NextFuture<'a> = impl Future<Output=Option<Self::Item<'a>>> where S: 'a;

    fn next<'a>(&'a mut self) -> Self::NextFuture<'a>
    where
        Self: 'a,
        Self: Unpin,
    {
        async {
            loop {
                if self.n == 0 {
                    self.n = self.step - 1;
                    return self.stream.next().await;
                } else {
                    self.n -= 1;
                    let _ = self.stream.next().await?;
                }
            }
        }
    }
}

pin_project! {
    #[derive(Clone, Debug)]
    #[must_use = "streams do nothing unless polled"]
    pub struct Chain<S, U> {
        #[pin]
        first: Fuse<S>,
        #[pin]
        second: Fuse<U>,
    }
}

impl<S, U> LendingStream for Chain<S, U>
where
    S: LendingStream + Unpin,
    for<'a> U: LendingStream<Item<'a> = S::Item<'a>> + Unpin + 'a,
{
    type Item<'a> = S::Item<'a> where S: 'a;
    type NextFuture<'a> = impl Future<Output=Option<Self::Item<'a>>> where S: 'a;

    fn next<'a>(&'a mut self) -> Self::NextFuture<'a>
    where
        Self: 'a,
        Self: Unpin,
    {
        async {
            match self.first.next().await {
                None => self.second.next().await,
                Some(x) => Some(x),
            }
        }
    }
}

pin_project! {
    #[derive(Clone, Debug)]
    #[must_use = "streams do nothing unless polled"]
    pub struct Cloned<S> {
        #[pin]
        stream: S,
    }
}

impl<S, T> LendingStream for Cloned<S>
where
    for<'a> S: LendingStream<Item<'a> = &'a T> + Unpin + 'a,
    T: Clone,
{
    type Item<'b> = T;
    type NextFuture<'b> = impl Future<Output = Option<Self::Item<'b>>>;

    fn next<'b>(&'b mut self) -> Self::NextFuture<'b>
    where
        Self: 'b,
        Self: Unpin,
    {
        async {
            let next = self.stream.next().await?;
            Some(next.clone())
        }
    }
}

pin_project! {
    #[derive(Clone, Debug)]
    #[must_use = "streams do nothing unless polled"]
    pub struct Copied<S> {
        #[pin]
        stream: S,
    }
}

impl<S, T> LendingStream for Copied<S>
where
    for<'a> S: LendingStream<Item<'a> = &'a T> + Unpin + 'a,
    T: Copy,
{
    type Item<'b> = T;
    type NextFuture<'b> = impl Future<Output = Option<Self::Item<'b>>>;

    fn next<'b>(&'b mut self) -> Self::NextFuture<'b>
    where
        Self: 'b,
        Self: Unpin,
    {
        async {
            let next = self.stream.next().await?;
            Some(*next)
        }
    }
}

pin_project! {
    #[derive(Clone, Debug)]
    #[must_use = "streams do nothing unless polled"]
    pub struct Scan<S, St, F> {
        #[pin]
        stream: S,
        state_f: (St, F),
    }
}

impl<S, St, F, B> LendingStream for Scan<S, St, F>
where
    S: LendingStream + Unpin,
    F: FnMut(&mut St, S::Item<'_>) -> Option<B>,
{
    type Item<'a> = B where F: 'a, St: 'a, S: 'a;
    type NextFuture<'a> = impl Future<Output=Option<Self::Item<'a>>> where F: 'a, St: 'a, S: 'a;

    fn next<'a>(&'a mut self) -> Self::NextFuture<'a>
    where
        Self: 'a,
        Self: Unpin,
    {
        async {
            let next = self.stream.next().await?;
            (self.state_f.1)(&mut self.state_f.0, next)
        }
    }
}

pin_project! {
    #[derive(Clone, Debug)]
    #[must_use = "streams do nothing unless polled"]
    pub struct Fuse<S> {
        #[pin]
        stream: S,
        done: bool,
    }
}

impl<S> LendingStream for Fuse<S>
where
    S: LendingStream + Unpin,
{
    type Item<'a> = S::Item<'a> where S: 'a;
    type NextFuture<'a> = impl Future<Output=Option<Self::Item<'a>>> where S: 'a;

    fn next<'a>(&'a mut self) -> Self::NextFuture<'a>
    where
        Self: 'a,
        Self: Unpin,
    {
        async {
            if self.done {
                None
            } else {
                match self.stream.next().await {
                    None => {
                        self.done = true;
                        None
                    }
                    Some(x) => Some(x),
                }
            }
        }
    }
}

#[cfg(feature = "polonius")]
pin_project! {
    #[derive(Clone, Debug)]
    #[must_use = "streams do nothing unless polled"]
    pub struct Cycle<S> {
        orig: S,
        #[pin]
        stream: S,
    }
}

#[cfg(feature = "polonius")]
impl<S> LendingStream for Cycle<S>
where
    S: LendingStream + Clone + Unpin,
{
    type Item<'a> = S::Item<'a> where S: 'a;
    type NextFuture<'a> = impl Future<Output = Option<Self::Item<'a>>> where S: 'a;

    fn next<'a>(&'a mut self) -> Self::NextFuture<'a>
    where
        Self: 'a,
        Self: Unpin,
    {
        async {
            match self.stream.next().await {
                None => {
                    self.stream = self.orig.clone();
                    self.stream.next().await
                }
                Some(x) => Some(x),
            }
        }
    }
}

pin_project! {
    #[derive(Clone, Debug)]
    #[must_use = "streams do nothing unless polled"]
    pub struct Enumerate<S> {
        #[pin]
        stream: S,
        i: usize,
    }
}

impl<S> LendingStream for Enumerate<S>
where
    S: LendingStream + Unpin,
{
    type Item<'a> = (usize, S::Item<'a>) where S: 'a;
    type NextFuture<'a> = impl Future<Output=Option<Self::Item<'a>>> where S: 'a;

    fn next<'a>(&'a mut self) -> Self::NextFuture<'a>
    where
        Self: 'a,
        Self: Unpin,
    {
        async {
            let next = self.stream.next().await?;
            let i = self.i;
            self.i += 1;
            Some((i, next))
        }
    }
}

pin_project! {
    #[derive(Clone, Debug)]
    #[must_use = "streams do nothing unless polled"]
    pub struct Inspect<S, F> {
        #[pin]
        stream: S,
        f: F,
    }
}

impl<S, F> LendingStream for Inspect<S, F>
where
    S: LendingStream + Unpin,
    for<'a> F: FnMut(&S::Item<'a>),
{
    type Item<'a> = S::Item<'a> where S: 'a, F: 'a;
    type NextFuture<'a> = impl Future<Output=Option<S::Item<'a>>> where S: 'a, F: 'a;

    fn next<'a>(&'a mut self) -> Self::NextFuture<'a>
    where
        Self: 'a,
        Self: Unpin,
    {
        async {
            let next = self.stream.next().await?;
            (self.f)(&next);
            Some(next)
        }
    }
}

pin_project! {
    #[derive(Clone, Debug)]
    #[must_use = "streams do nothing unless polled"]
    pub struct Zip<A: LendingStream, B> {
        #[pin]
        first: A,
        #[pin]
        second: B,
    }
}

impl<A, B> LendingStream for Zip<A, B>
where
    A: LendingStream + Unpin,
    for<'a> B: LendingStream<Item<'a> = A::Item<'a>> + Unpin + 'a,
{
    type Item<'a> = (A::Item<'a>, B::Item<'a>) where A: 'a, B: 'a;
    type NextFuture<'a> = impl Future<Output=Option<Self::Item<'a>>> where A: 'a, B: 'a;

    fn next<'a>(&'a mut self) -> Self::NextFuture<'a>
    where
        Self: 'a,
        Self: Unpin,
    {
        async {
            let a = self.first.next().await?;
            let b = self.second.next().await?;
            Some((a, b))
        }
    }
}

pin_project! {
    #[derive(Clone, Debug)]
    #[must_use = "streams do nothing unless polled"]
    pub struct Or<S1, S2> {
        #[pin]
        stream1: S1,
        #[pin]
        stream2: S2,
    }
}

impl<T, S1, S2> LendingStream for Or<S1, S2>
where
    for<'a> S1: LendingStream<Item<'a> = T> + Unpin + 'a,
    for<'a> S2: LendingStream<Item<'a> = T> + Unpin + 'a,
{
    type Item<'a> = S1::Item<'a> where S1: 'a, S2: 'a;
    type NextFuture<'a> = impl Future<Output=Option<S1::Item<'a>>> where S1: 'a, S2: 'a;

    fn next<'a>(&'a mut self) -> Self::NextFuture<'a>
    where
        Self: 'a,
        Self: Unpin,
    {
        async {
            let next = self.stream1.next().await;
            match next {
                Some(next) => Some(next),
                None => self.stream2.next().await,
            }
        }
    }
}

pin_project! {
    #[derive(Clone, Debug)]
    #[must_use = "streams do nothing unless polled"]
    pub struct Race<S1, S2> {
        #[pin]
        stream1: S1,
        #[pin]
        stream2: S2,
    }
}

impl<T, S1, S2> LendingStream for Race<S1, S2>
where
    for<'a> S1: LendingStream<Item<'a> = T> + Unpin + 'a,
    for<'a> S2: LendingStream<Item<'a> = T> + Unpin + 'a,
{
    type Item<'a> = S1::Item<'a> where S1: 'a, S2: 'a;
    type NextFuture<'a> = impl Future<Output=Option<S1::Item<'a>>> where S1: 'a, S2: 'a;

    fn next<'a>(&'a mut self) -> Self::NextFuture<'a>
    where
        Self: 'a,
        Self: Unpin,
    {
        async {
            if fastrand::bool() {
                let next = self.stream1.next().await;
                match next {
                    Some(next) => Some(next),
                    None => self.stream2.next().await,
                }
            } else {
                let next = self.stream2.next().await;
                match next {
                    Some(next) => Some(next),
                    None => self.stream1.next().await,
                }
            }
        }
    }
}
