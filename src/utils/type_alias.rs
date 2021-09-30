use std::{
    pin::Pin,
    future::Future
};

pub type BoxedFuture<'a, T> = Pin<Box<dyn 'a + Send + Future<Output = T>>>;

pub type U8Callback<U> = for<'a> fn(&mut U, &'a [u8]) -> BoxedFuture<'a, bool>;