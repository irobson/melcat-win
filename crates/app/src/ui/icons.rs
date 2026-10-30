//! Tray icons: the four PNGs (white or black Mel for a dark or light taskbar,
//! active or paused) are decoded with WIC, scaled to the small-icon size of
//! the current DPI and turned into `HICON`s.

use windows::Win32::Graphics::Gdi::{
    CreateBitmap, CreateDIBSection, DeleteObject, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS, HBITMAP,
};
use windows::Win32::Graphics::Imaging::{
    CLSID_WICImagingFactory, GUID_WICPixelFormat32bppPBGRA, IWICImagingFactory, WICBitmapDitherTypeNone,
    WICBitmapInterpolationModeFant, WICBitmapPaletteTypeCustom, WICDecodeMetadataCacheOnDemand,
};
use windows::Win32::System::Com::{CoCreateInstance, CLSCTX_INPROC_SERVER};
use windows::Win32::UI::WindowsAndMessaging::{CreateIconIndirect, DestroyIcon, HICON, ICONINFO};

const ACTIVE_DARK: &[u8] = include_bytes!("../../resources/tray/tray-active-dark.png");
const ACTIVE_LIGHT: &[u8] = include_bytes!("../../resources/tray/tray-active-light.png");
const PAUSED_DARK: &[u8] = include_bytes!("../../resources/tray/tray-paused-dark.png");
const PAUSED_LIGHT: &[u8] = include_bytes!("../../resources/tray/tray-paused-light.png");

/// The PNG for a taskbar theme and state. The "dark" artwork (a white Mel)
/// is for a dark taskbar; "paused" is the same glyph, dimmed, shown whenever
/// Mel is not playing (off, napping), like the macOS app's dimmed icon.
pub(crate) fn tray_png(dark_taskbar: bool, active: bool) -> &'static [u8] {
    match (dark_taskbar, active) {
        (true, true) => ACTIVE_DARK,
        (false, true) => ACTIVE_LIGHT,
        (true, false) => PAUSED_DARK,
        (false, false) => PAUSED_LIGHT,
    }
}

/// Undoes premultiplied alpha in place (BGRA): icons expect straight alpha.
pub(crate) fn unpremultiply(bgra: &mut [u8]) {
    for px in bgra.as_chunks_mut::<4>().0 {
        let a = px[3] as u32;
        if a == 0 {
            px[0] = 0;
            px[1] = 0;
            px[2] = 0;
        } else if a < 255 {
            for c in &mut px[..3] {
                *c = ((*c as u32 * 255 + a / 2) / a).min(255) as u8;
            }
        }
    }
}

/// An owned `HICON`, destroyed on drop.
pub(crate) struct Icon(HICON);

impl Icon {
    pub(crate) fn handle(&self) -> HICON {
        self.0
    }
}

impl Drop for Icon {
    fn drop(&mut self) {
        // SAFETY: we created the icon with CreateIconIndirect and own it.
        unsafe {
            let _ = DestroyIcon(self.0);
        }
    }
}

/// Decodes `png` and makes a `size` x `size` icon from it.
pub(crate) fn icon_from_png(png: &[u8], size: u32) -> windows::core::Result<Icon> {
    let size = size.clamp(8, 256);
    let mut pixels = decode_scaled(png, size)?;
    // WIC scales premultiplied pixels cleanly; icons want straight alpha.
    unpremultiply(&mut pixels);
    icon_from_bgra(&pixels, size)
}

/// Decodes `png` into premultiplied BGRA scaled to `size` x `size`.
fn decode_scaled(png: &[u8], size: u32) -> windows::core::Result<Vec<u8>> {
    // SAFETY: plain WIC calls; `png` outlives the stream that reads it (both end here).
    unsafe {
        let factory: IWICImagingFactory = CoCreateInstance(&CLSID_WICImagingFactory, None, CLSCTX_INPROC_SERVER)?;
        let stream = factory.CreateStream()?;
        stream.InitializeFromMemory(png)?;
        let decoder = factory.CreateDecoderFromStream(&stream, std::ptr::null(), WICDecodeMetadataCacheOnDemand)?;
        let frame = decoder.GetFrame(0)?;
        let converter = factory.CreateFormatConverter()?;
        converter.Initialize(
            &frame,
            &GUID_WICPixelFormat32bppPBGRA,
            WICBitmapDitherTypeNone,
            None,
            0.0,
            WICBitmapPaletteTypeCustom,
        )?;
        let scaler = factory.CreateBitmapScaler()?;
        scaler.Initialize(&converter, size, size, WICBitmapInterpolationModeFant)?;
        let stride = size * 4;
        let mut pixels = vec![0u8; (stride * size) as usize];
        scaler.CopyPixels(std::ptr::null(), stride, &mut pixels)?;
        Ok(pixels)
    }
}

/// Owns a GDI bitmap until drop.
struct Bitmap(HBITMAP);

impl Drop for Bitmap {
    fn drop(&mut self) {
        // SAFETY: we created the bitmap; CreateIconIndirect copies it, so it can go.
        unsafe {
            let _ = DeleteObject(self.0.into());
        }
    }
}

/// Builds an alpha icon from straight-alpha, top-down BGRA pixels.
fn icon_from_bgra(pixels: &[u8], size: u32) -> windows::core::Result<Icon> {
    let side = size as i32;
    // SAFETY: the DIB section is size x size x 4 bytes and `pixels` has exactly that many;
    // the mask buffer is word-aligned rows of zero bits as CreateBitmap expects.
    unsafe {
        let info = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: side,
                biHeight: -side,
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                ..Default::default()
            },
            ..Default::default()
        };
        let mut bits = std::ptr::null_mut();
        let color = Bitmap(CreateDIBSection(None, &info, DIB_RGB_COLORS, &mut bits, None, 0)?);
        if bits.is_null() || pixels.len() < (size * size * 4) as usize {
            return Err(windows::core::Error::from_thread());
        }
        std::ptr::copy_nonoverlapping(pixels.as_ptr(), bits as *mut u8, (size * size * 4) as usize);
        let mask_stride = (size as usize).div_ceil(16) * 2;
        let mask_bits = vec![0u8; mask_stride * size as usize];
        let mask = CreateBitmap(side, side, 1, 1, Some(mask_bits.as_ptr() as *const _));
        if mask.is_invalid() {
            return Err(windows::core::Error::from_thread());
        }
        let mask = Bitmap(mask);
        let icon_info = ICONINFO { fIcon: true.into(), xHotspot: 0, yHotspot: 0, hbmMask: mask.0, hbmColor: color.0 };
        Ok(Icon(CreateIconIndirect(&icon_info)?))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_variant_is_a_png() {
        for dark in [false, true] {
            for active in [false, true] {
                assert!(tray_png(dark, active).starts_with(b"\x89PNG\r\n\x1a\n"));
            }
        }
        assert_ne!(tray_png(true, true), tray_png(false, true));
        assert_ne!(tray_png(true, true), tray_png(true, false));
    }

    #[test]
    fn unpremultiply_restores_straight_alpha() {
        let mut px = [64, 32, 0, 128, 10, 20, 30, 0, 1, 2, 3, 255];
        unpremultiply(&mut px);
        assert_eq!(px, [128, 64, 0, 128, 0, 0, 0, 0, 1, 2, 3, 255]);
    }
}
