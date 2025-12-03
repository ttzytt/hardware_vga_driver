use core::iter::Step;
pub trait PrimInt:
    num_traits::PrimInt + Step
{

}

impl PrimInt for u8 {}
impl PrimInt for u16 {}
impl PrimInt for u32 {}
impl PrimInt for u64 {}
impl PrimInt for u128 {}
impl PrimInt for i8 {}
impl PrimInt for i16 {}
impl PrimInt for i32 {}
impl PrimInt for i64 {}
impl PrimInt for i128 {}

