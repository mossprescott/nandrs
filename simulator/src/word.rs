/// Integer values of arbitrary bit width up to 64, stored as u64 for efficiency during simulation,
/// with conversion to and from signed and unsigned values, masked to the correct size.
///
/// The idea is: during simulation, bits are bits; everything is stored in u64, which is assumed to be
/// efficiently handled by the host.

use std::fmt;
use std::hash::{Hash, Hasher};
use std::marker::PhantomData;

use crate::bool::False;
use crate::nat::{Cmp, Compare, IsGreater, IsLess, Nat, N1, N8, N16, N32, N64};

/// A type-level word size that can handle at least `Min` bits, and will fit in our usual storage word
/// (u64).
pub trait StorableFor<Min: Nat>: Nat + Cmp<N64> + Cmp<Min>
    + IsGreater<Compare<Self, N64>, Output = False>
    + IsLess<Compare<Self, Min>, Output = False>
{}

impl<Width, Min> StorableFor<Min> for Width
where
    Min: Nat,
    Width: Nat + Cmp<N64> + Cmp<Min>
        + IsGreater<Compare<Width, N64>, Output = False>
        + IsLess<Compare<Width, Min>, Output = False>,
{}

/// A type-level word size that will fit in our usual storage word (u64).
pub trait Storable: Nat + Cmp<N64> + IsGreater<Compare<Self, N64>, Output = False> {}

impl<Width> Storable for Width
where
    Width: Nat + Cmp<N64>
        + IsGreater<Compare<Width, N64>, Output = False>,
{}

/// Store bits which can be treated as a signed or unsigned value of the specified width.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Word<Width: Storable> {
    val: u64,
    _width: PhantomData<Width>,
}

impl<Width: Storable> Hash for Word<Width> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.val.hash(state);
    }
}

impl<Width: Nat + Storable> Word<Width> {
    /// Unsafe: accept any bits.
    pub fn new(val: u64) -> Self {
        Word { val: val & Self::mask(), _width: PhantomData }
    }

    fn mask() -> u64 {
        let w = Width::as_int();
        if w >= 64 { u64::MAX } else { (1u64 << w) - 1 }
    }

    /// Interpret the bits as an unsigned value. Always u64 for convenience.
    pub fn unsigned(&self) -> u64 {
        self.val & Self::mask()
    }

    /// Interpret the bits as a signed value. Always i64 for convenience.
    pub fn signed(&self) -> i64 {
        let w = Width::as_int();
        let masked = self.unsigned();
        if w >= 64 {
            masked as i64
        } else {
            let sign_bit = 1u64 << (w - 1);
            if masked & sign_bit != 0 {
                // Sign-extend: fill upper bits with 1s
                (masked | !Self::mask()) as i64
            } else {
                masked as i64
            }
        }
    }
}

impl<Width: Storable> fmt::Display for Word<Width> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let w = Width::as_int();
        let hex_digits = (w + 3) / 4;
        let signed = self.signed();
        let unsigned = self.unsigned();
        if signed < 0 {
            write!(f, "{} (0x{:0>width$X}; {})", signed, unsigned, unsigned, width = hex_digits)
        } else {
            write!(f, "{} (0x{:0>width$X})", signed, unsigned, width = hex_digits)
        }
    }
}

/// Safe conversion for a single bit
impl<Width: Storable + StorableFor<N1>> From<bool> for Word<Width> {
    fn from(val: bool) -> Word<Width> { Word::<Width>::new(val as u64) }
}

/// Safe conversion for 16-bit signed values
impl<Width: Storable + StorableFor<N16>> From<i16> for Word<Width> {
    fn from(val: i16) -> Word<Width> { Word::<Width>::new(val as u16 as u64) }
}

/// Safe conversion for 16-bit unsigned values
impl<Width: Storable + StorableFor<N16>> From<u16> for Word<Width> {
    fn from(val: u16) -> Word<Width> { Word::<Width>::new(val as u64) }
}

/// Safe conversion for 32-bit signed values
impl<Width: Storable + StorableFor<N32>> From<i32> for Word<Width> {
    fn from(val: i32) -> Word<Width> { Word::<Width>::new(val as u32 as u64) }
}

/// Safe conversion for 32-bit unsigned values
impl<Width: Storable + StorableFor<N32>> From<u32> for Word<Width> {
    fn from(val: u32) -> Word<Width> { Word::<Width>::new(val as u64) }
}

/// Safe conversion for 64-bit signed values
impl<Width: Storable + StorableFor<N64>> From<i64> for Word<Width> {
    fn from(val: i64) -> Word<Width> { Word::<Width>::new(val as u64) }
}

/// Safe conversion for 64-bit unsigned values
impl<Width: Storable + StorableFor<N64>> From<u64> for Word<Width> {
    fn from(val: u64) -> Word<Width> { Word::<Width>::new(val) }
}

pub type Word8 = Word<N8>;
pub type Word16 = Word<N16>;
pub type Word32 = Word<N32>;
pub type Word64 = Word<N64>;


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unsigned16() {
        assert_eq!(Word16::new(0x0001).unsigned(), 1);
        assert_eq!(Word16::new(0xFFFF).unsigned(), 65535);

        // Extra bits ignored:
        assert_eq!(Word16::new(0xFFFFFFFFF).unsigned(), 65535);
    }

    #[test]
    fn signed16() {
        assert_eq!(Word16::new(0x0001).signed(), 1);
        assert_eq!(Word16::new(0xFFFF).signed(), -1);

        // Extra bits ignored:
        assert_eq!(Word16::new(0xFFFFFFFFF).signed(), -1);
    }

    #[test]
    fn convert16() {
        let x: Word16 = (0xFFFFu16).into();
        assert_eq!(x.unsigned(), 65535);

        let y: Word16 = (-1i16).into();
        assert_eq!(y.signed(), -1);

        // Type error: can't store a 32-bit value in Word16
        // let z: Word16 = (-1i32).into();
    }

    #[test]
    fn display16() {
        let x: Word16 = (1u16).into();
        assert_eq!(format!("{x}"), "1 (0x0001)");

        let y: Word16 = (-1i16).into();
        assert_eq!(format!("{y}"), "-1 (0xFFFF; 65535)");
    }

    #[test]
    fn unsigned64() {
        assert_eq!(Word64::new(0x0001).unsigned(), 1);
        assert_eq!(Word64::new(u64::MAX).unsigned(), u64::MAX);
    }

    #[test]
    fn signed64() {
        assert_eq!(Word64::new(0x0001).signed(), 1);
        assert_eq!(Word64::new(u64::MAX).signed(), -1);
    }

    #[test]
    fn convert64() {
        let x: Word64 = (u64::MAX).into();
        assert_eq!(x.unsigned(), u64::MAX);

        let y: Word64 = (-1i64).into();
        assert_eq!(y.signed(), -1);

        // // Type error: can't store a 128-bit value in Word64
        // let z: Word64 = (-1i128).into();
    }
}
