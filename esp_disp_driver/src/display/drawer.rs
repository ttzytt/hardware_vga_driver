use crate::display::pix_writer::PixelWriter;
use crate::utils::PrimInt;
pub struct Drawer<'a, AddrT: PrimInt, ColorT: PrimInt, PW>
where
    PW: PixelWriter<AddrT, ColorT>,
{
    pixel_writer: &'a mut PW,
    _marker_color: core::marker::PhantomData<ColorT>,
    _marker_addr: core::marker::PhantomData<AddrT>,
}

impl<'a, AddrT: PrimInt, ColorT: PrimInt, PW>
Drawer<'a, AddrT, ColorT, PW> where
    PW: PixelWriter<AddrT, ColorT>
{
    pub fn new(pixel_writer: &'a mut PW) -> Self {
        Drawer {
            pixel_writer,
            _marker_color: core::marker::PhantomData,
            _marker_addr: core::marker::PhantomData,
        }
    }

    pub fn fill_screen(&mut self, color: ColorT) {
        let ((i_min, i_max), (j_min, j_max)) = self.pixel_writer.addr_range();
        for i in i_min..=i_max {
            for j in j_min..=j_max {
                self.pixel_writer.write_pixel(i, j, color);
            }
        }
    }

    pub fn draw_rectangle(
        &mut self,
        i_start: AddrT,
        j_start: AddrT,
        width: AddrT,
        height: AddrT,
        color: ColorT,
    ) {
        let i_end = i_start + width - AddrT::one();
        let j_end = j_start + height - AddrT::one();
        for i in i_start..=i_end {
            for j in j_start..=j_end {
                self.pixel_writer.write_pixel(i, j, color);
            }
        }
    }

    pub fn write_pixel(&mut self, i: AddrT, j: AddrT, color: ColorT) {
        self.pixel_writer.write_pixel(i, j, color);
    }
}
