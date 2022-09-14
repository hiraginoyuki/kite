pub mod convert {
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

    pub trait FromInner<Inner> {
        fn from_inner(inner: Inner) -> Self;
    }

    pub trait TryFromInner<Inner>: Sized {
        type Error;
        fn try_from_inner(inner: Inner) -> Result<Self, Self::Error>;
    }
}
