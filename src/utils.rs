use std::ops::{Add, Sub};
use std::{cmp::PartialOrd, convert::TryInto};

/// helper trait for [`AddOffset`]
pub trait Negative: Copy + Sized {
    fn is_negative_(self) -> bool;
    fn negate(self) -> Option<Self>;
}

pub trait AddOffset:
    Copy + Sized + Add<Output = Self> + Sub<Output = Self> + PartialOrd<Self>
{
    type Offset: TryInto<Self> + Negative;

    #[inline]
    fn add_offset(self, x: Self::Offset) -> Option<Self> {
        if x.is_negative_() {
            let xn: Self = x.negate()?.try_into().ok()?;
            if xn <= self {
                Some(self - xn)
            } else {
                None
            }
        } else {
            let x: Self = x.try_into().ok()?;
            Some(self + x)
        }
    }
}

macro_rules! ximpl {
    ($($ut:ty: $it:ty),+ $(,)?) => {
        $(
            impl Negative for $it {
                #[inline(always)]
                fn is_negative_(self) -> bool {
                    self.is_negative()
                }
                #[inline(always)]
                fn negate(self) -> Option<Self> {
                    let (ret, ovf) = self.overflowing_neg();
                    if ovf {
                        None
                        } else {
                        Some(ret)
                    }
                }
            }
            impl AddOffset for $ut {
                type Offset = $it;
            }
        )+
    }
}

ximpl! {
    u8: i8,
    u16: i16,
    u32: i32,
    u64: i64,
    usize: isize,
}
