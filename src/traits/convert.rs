pub trait AsInner {
    type Inner: ?Sized;
    fn as_inner(&self) -> &Self::Inner;
}

pub trait AsInnerMut {
    type Inner: ?Sized;
    fn as_inner_mut(&mut self) -> &mut Self::Inner;
}

pub trait IntoInner {
    type Inner;
    fn into_inner(self) -> Self::Inner;
}

pub trait FromInner: Sized {
    type Inner;
    fn from_inner(inner: Self::Inner) -> Self;
}

pub trait TryFromInner: Sized {
    type Inner;
    type Error;
    fn try_from_inner(inner: Self::Inner) -> Result<Self, Self::Error>;
}
