/// Type-level natural numbers.
///
/// https://gist.github.com/bodil/a6f61e139fdf892b1a632c55f7cffc8b

use std::ops::{Add, Sub};
use std::cmp::Ordering;
use std::marker::PhantomData;
use std::fmt;

use crate::bool::*;

pub trait Nat {
    fn new() -> Self;
    fn as_int() -> usize;
}

#[derive(Copy, Clone, PartialEq, Eq)]
pub struct Zero;
impl Nat for Zero {
    fn new() -> Self {
        Zero
    }
    fn as_int() -> usize {
        0
    }
}

#[derive(Copy, Clone, PartialEq, Eq)]
pub struct Succ<N> {
    n: PhantomData<N>,
}
impl<N> Nat for Succ<N>
where
    N: Nat,
{
    fn new() -> Self {
        Succ { n: PhantomData }
    }
    fn as_int() -> usize {
        1 + N::as_int()
    }
}

pub trait Pos: Nat {}
impl<N> Pos for Succ<N>
where
    N: Nat,
{
}

// Addition

pub type Sum<L, R> = <L as Add<R>>::Output;

impl<N: Nat> Add<N> for Zero {
    type Output = N;
    fn add(self, n: N) -> Self::Output {
        n
    }
}

impl<L: Nat, R: Add<L>> Add<R> for Succ<L> where <R as Add<L>>::Output: Nat {
    type Output = Succ<<R as Add<L>>::Output>;
    fn add(self, _: R) -> Self::Output {
        <Self::Output as Nat>::new()
    }
}

// Subtraction

pub type Diff<L, R> = <L as Sub<R>>::Output;

impl Sub<Zero> for Zero {
    type Output = Self;
    fn sub(self, _: Zero) -> Self::Output {
        self
    }
}

impl<N: Nat> Sub<Zero> for Succ<N> {
    type Output = Self;
    fn sub(self, _: Zero) -> Self::Output {
        self
    }
}

impl<L: Nat + Sub<R>, R: Nat> Sub<Succ<R>> for Succ<L> where <L as Sub<R>>::Output: Nat {
    type Output = <L as Sub<R>>::Output;
    fn sub(self, _: Succ<R>) -> Self::Output {
        <Self::Output as Nat>::new()
    }
}

pub type Pred<N> = Diff<N, Succ<Zero>>;

// Ordering

pub trait Cmp<Other> {
    type Output: Order;
}

pub type Compare<L, R> = <L as Cmp<R>>::Output;

pub trait Order {
    fn as_ordering() -> Ordering;
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct Less;
impl Less {
    pub fn new() -> Less {
        Less
    }
}
impl Order for Less {
    fn as_ordering() -> Ordering {
        Ordering::Less
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct Equal;
impl Equal {
    pub fn new() -> Equal {
        Equal
    }
}
impl Order for Equal {
    fn as_ordering() -> Ordering {
        Ordering::Equal
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct Greater;
impl Greater {
    pub fn new() -> Greater {
        Greater
    }
}
impl Order for Greater {
    fn as_ordering() -> Ordering {
        Ordering::Greater
    }
}

impl Cmp<Zero> for Zero {
    type Output = Equal;
}

impl<N: Nat> Cmp<Zero> for Succ<N> {
    type Output = Greater;
}

impl<N: Nat> Cmp<Succ<N>> for Zero {
    type Output = Less;
}

impl<L: Nat + Cmp<R>, R: Nat> Cmp<Succ<R>> for Succ<L> {
    type Output = <L as Cmp<R>>::Output;
}

impl<R: Nat> PartialOrd<R> for Zero
where
    Zero: Cmp<R> + PartialEq<R>,
{
    fn partial_cmp(&self, _: &R) -> Option<Ordering> {
        Some(<Zero as Cmp<R>>::Output::as_ordering())
    }
}

impl<L: Nat + Cmp<R>, R: Nat> PartialOrd<R> for Succ<L>
where
    Succ<L>: PartialEq<R>,
{
    fn partial_cmp(&self, _: &R) -> Option<Ordering> {
        Some(<L as Cmp<R>>::Output::as_ordering())
    }
}

pub trait IsLess<C> {
    type Output: Bool;
}
impl<L> IsLess<Less> for L {
    type Output = True;
}
impl<L> IsLess<Equal> for L {
    type Output = False;
}
impl<L> IsLess<Greater> for L {
    type Output = False;
}

pub trait IsEqual<C> {
    type Output: Bool;
}
impl<L> IsEqual<Less> for L {
    type Output = False;
}
impl<L> IsEqual<Equal> for L {
    type Output = True;
}
impl<L> IsEqual<Greater> for L {
    type Output = False;
}

pub trait IsGreater<C> {
    type Output: Bool;
}
impl<L> IsGreater<Less> for L {
    type Output = False;
}
impl<L> IsGreater<Equal> for L {
    type Output = False;
}
impl<L> IsGreater<Greater> for L {
    type Output = True;
}

pub trait LT<R> {
    type Output: Bool;
}
impl<L, R> LT<R> for L
where
    L: Cmp<R> + IsLess<Compare<L, R>>,
{
    type Output = <L as IsLess<Compare<L, R>>>::Output;
}

pub trait EQ<R> {
    type Output: Bool;
}
impl<L, R> EQ<R> for L
where
    L: Cmp<R> + IsEqual<Compare<L, R>>,
{
    type Output = <L as IsEqual<Compare<L, R>>>::Output;
}

pub trait GT<R> {
    type Output: Bool;
}
impl<L, R> GT<R> for L
where
    L: Cmp<R> + IsGreater<Compare<L, R>>,
{
    type Output = <L as IsGreater<Compare<L, R>>>::Output;
}

pub type Lt<L, R> = <L as LT<R>>::Output;
pub type Eq<L, R> = <L as EQ<R>>::Output;
pub type Gt<L, R> = <L as GT<R>>::Output;



// Literals

pub type N0 = Zero;
pub type N1 = Succ<N0>;
pub type N2 = Succ<N1>;
pub type N3 = Succ<N2>;
pub type N4 = Succ<N3>;
pub type N5 = Succ<N4>;
pub type N6 = Succ<N5>;
pub type N7 = Succ<N6>;
pub type N8 = Succ<N7>;
pub type N9 = Succ<N8>;
pub type N10 = Succ<N9>;
pub type N11 = Succ<N10>;
pub type N12 = Succ<N11>;
pub type N13 = Succ<N12>;
pub type N14 = Succ<N13>;
pub type N15 = Succ<N14>;
pub type N16 = Succ<N15>;
pub type N17 = Succ<N16>;
pub type N18 = Succ<N17>;
pub type N19 = Succ<N18>;
pub type N20 = Succ<N19>;
pub type N21 = Succ<N20>;
pub type N22 = Succ<N21>;
pub type N23 = Succ<N22>;
pub type N24 = Succ<N23>;
pub type N25 = Succ<N24>;
pub type N26 = Succ<N25>;
pub type N27 = Succ<N26>;
pub type N28 = Succ<N27>;
pub type N29 = Succ<N28>;
pub type N30 = Succ<N29>;
pub type N31 = Succ<N30>;
pub type N32 = Succ<N31>;
pub type N33 = Succ<N32>;
pub type N34 = Succ<N33>;
pub type N35 = Succ<N34>;
pub type N36 = Succ<N35>;
pub type N37 = Succ<N36>;
pub type N38 = Succ<N37>;
pub type N39 = Succ<N38>;
pub type N40 = Succ<N39>;
pub type N41 = Succ<N40>;
pub type N42 = Succ<N41>;
pub type N43 = Succ<N42>;
pub type N44 = Succ<N43>;
pub type N45 = Succ<N44>;
pub type N46 = Succ<N45>;
pub type N47 = Succ<N46>;
pub type N48 = Succ<N47>;
pub type N49 = Succ<N48>;
pub type N50 = Succ<N49>;
pub type N51 = Succ<N50>;
pub type N52 = Succ<N51>;
pub type N53 = Succ<N52>;
pub type N54 = Succ<N53>;
pub type N55 = Succ<N54>;
pub type N56 = Succ<N55>;
pub type N57 = Succ<N56>;
pub type N58 = Succ<N57>;
pub type N59 = Succ<N58>;
pub type N60 = Succ<N59>;
pub type N61 = Succ<N60>;
pub type N62 = Succ<N61>;
pub type N63 = Succ<N62>;
pub type N64 = Succ<N63>;


impl fmt::Debug for Zero {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(f, "0")
    }
}

impl<N: Nat> fmt::Debug for Succ<N> {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(f, "{}", Self::as_int())
    }
}



// Tests

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn it_works() {
        assert_eq!(N0::new() + N0::new(), N0::new());
        assert_eq!(N0::new() + N2::new(), N2::new());
        assert_eq!(N2::new() + N0::new(), N2::new());
        assert_eq!(N2::new() + N2::new(), N4::new());
        assert_eq!(N2::new() - N0::new(), N2::new());
        assert_eq!(N0::new() - N0::new(), N0::new());
        assert_eq!(N6::new() - N4::new(), N2::new());
        assert_eq!(
            format!("{:?}", N2::new() + N2::new()),
            format!("{:?}", N4::new())
        );

        assert_eq!(Greater::new(), Compare::<N4, N2>::new());
        assert_eq!(Less::new(), Compare::<N4, N6>::new());
        assert_eq!(Equal::new(), Compare::<N4, N4>::new());

        assert_eq!(True, Lt::<N2, N4>::new());
        assert_eq!(False, Lt::<N4, N2>::new());
        assert_eq!(False, Lt::<N2, N2>::new());

        assert_eq!(False, Gt::<N2, N4>::new());
        assert_eq!(True, Gt::<N4, N2>::new());
        assert_eq!(False, Gt::<N2, N2>::new());

        assert_eq!(False, Eq::<N2, N4>::new());
        assert_eq!(False, Eq::<N4, N2>::new());
        assert_eq!(True, Eq::<N2, N2>::new());
    }
}