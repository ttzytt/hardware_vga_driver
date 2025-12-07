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

#[macro_export]
macro_rules! anypins_from_peri {
    ($perip:expr; $($n:literal),+ $(,)?) => {{
        [
            $(
                paste::paste! { $perip.[<GPIO $n>].into() }
            ),+
        ]
    }};
}