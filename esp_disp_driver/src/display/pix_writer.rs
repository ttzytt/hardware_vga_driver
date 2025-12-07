use crate::utils::PrimInt;

pub trait PixelWriter<AddrT : PrimInt, ColorT : PrimInt>{
    fn write_pixel(&mut self, i: AddrT, j: AddrT, color: ColorT);
    fn addr_range(&self) -> ((AddrT, AddrT), (AddrT, AddrT));
    // ((i_min, i_max), (j_min, j_max))
    fn color_range(&self) -> (ColorT, ColorT);
}
