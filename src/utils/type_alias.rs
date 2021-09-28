use std::{
    pin::Pin,
    future::Future
};

pub type BoxedFuture<'a, T> = Pin<Box<dyn 'a + Send + Future<Output = T>>>;