use std::{
    error::Error,
    fmt::{Debug, Display, Formatter},
    marker::PhantomData,
};

pub struct NeverError {
    _mark: PhantomData<()>,
}

impl Debug for NeverError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "NeverError")
    }
}

impl Display for NeverError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "NeverError")
    }
}

impl Error for NeverError {}
