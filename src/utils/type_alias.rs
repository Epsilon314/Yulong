use std::{
    pin::Pin,
    future::Future
};

pub type FutureRet<'a, T> = Pin<Box<dyn 'a + Send + Future<Output = T>>>;