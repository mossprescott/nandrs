/// Type-level booleans.
///
/// <https://gist.github.com/bodil/a6f61e139fdf892b1a632c55f7cffc8b>

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct True;

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct False;

pub trait Bool {
    fn new() -> Self;
}
impl Bool for True {
    fn new() -> True {
        True
    }
}
impl Bool for False {
    fn new() -> False {
        False
    }
}

pub trait IsTrue {}
impl IsTrue for True {}

pub trait IsFalse {}
impl IsFalse for False {}
