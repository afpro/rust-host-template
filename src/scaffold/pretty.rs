use std::fmt::{Debug, Display, Formatter};

pub struct Pretty<T>(pub T);
pub struct PrettyRef<'a, T: ?Sized>(pub &'a T);
pub struct PrettyOpt<T>(pub Option<T>);

impl<T> Display for Pretty<T>
where
    T: Display,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:#}", self.0)
    }
}

impl<T> Debug for Pretty<T>
where
    T: Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:#?}", self.0)
    }
}

impl<'a, T> Display for PrettyRef<'a, T>
where
    T: Display,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:#}", self.0)
    }
}

impl<'a, T> Debug for PrettyRef<'a, T>
where
    T: Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:#?}", self.0)
    }
}

impl<T> Display for PrettyOpt<T>
where
    T: Display,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self.0.as_ref() {
            Some(v) => write!(f, "{:#}", v),
            None => write!(f, "none"),
        }
    }
}

impl<T> Debug for PrettyOpt<T>
where
    T: Debug,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self.0.as_ref() {
            Some(v) => write!(f, "{:#?}", v),
            None => write!(f, "none"),
        }
    }
}
