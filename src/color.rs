use std;
use xcb::Connection;
use xcb::xproto;
use failure::{Error, err_msg};

#[derive(Clone, Copy, PartialEq)]
pub struct RGB {
    pub r: u8,
    pub g: u8,
    pub b: u8
}

pub const BLACK: RGB = RGB { r: 0, g: 0, b: 0 };
pub const WHITE: RGB = RGB { r: 0xff, g: 0xff, b: 0xff };

impl RGB {
    pub const fn new(r: u8, g: u8, b: u8) -> RGB {
        RGB { r, g, b }
    }

    pub fn is_compactable(self) -> bool {
        fn compact(n: u8) -> bool {
            (n >> 4) == (n & 0xf)
        }
        compact(self.r) && compact(self.g) && compact(self.b)
    }

    pub fn is_dark(self) -> bool {
        self.distance(BLACK) < self.distance(WHITE)
    }

    pub fn distance(self, other: RGB) -> f32 {
        ((f32::from(other.r) - f32::from(self.r)).powi(2) +
         (f32::from(other.g) - f32::from(self.g)).powi(2) +
         (f32::from(other.b) - f32::from(self.b)).powi(2))
            .sqrt()
    }

    pub fn interpolate(self, other: RGB, amount: f32) -> RGB {
        fn lerp(a: u8, b: u8, x: f32) -> u8 {
            ((1.0 - x) * f32::from(a) + x * f32::from(b)).ceil() as u8
        }
        RGB {
            r: lerp(self.r, other.r, amount),
            g: lerp(self.g, other.g, amount),
            b: lerp(self.b, other.b, amount)
        }
    }

    pub fn lighten(self, amount: f32) -> RGB {
        self.interpolate(WHITE, amount)
    }

    pub fn darken(self, amount: f32) -> RGB {
        self.interpolate(BLACK, amount)
    }
}

impl From<RGB> for u32 {
    fn from(color: RGB) -> u32 {
        u32::from(color.r) << 16 | u32::from(color.g) << 8 | u32::from(color.b)
    }
}

pub fn window_color_at_point(conn: &Connection, window: xproto::Window, (x, y): (i16, i16))
                         -> Result<RGB, Error> {
    let reply = xproto::get_image(conn,
                                  xproto::IMAGE_FORMAT_Z_PIXMAP as u8,
                                  window,
                                  x, y, 1, 1,
                                  std::u32::MAX)
        .get_reply()?;
    if reply.depth() != 24 {
        // TODO: Figure out what to do with these
        return Err(err_msg("Unsupported color depth"));
    }
    let data = reply.data();
    let r = data[2];
    let g = data[1];
    let b = data[0];
    Ok(RGB::new(r, g, b))
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct HSL {
    pub h: f32,
    pub s: f32,
    pub l: f32
}

impl HSL {
    // Source: https://en.wikipedia.org/wiki/HSL_and_HSV#From_RGB
    pub fn from_rgb(rgb: RGB) -> HSL {
        let r: f32 = f32::from(rgb.r) / 255.0;
        let g: f32 = f32::from(rgb.g) / 255.0;
        let b: f32 = f32::from(rgb.b) / 255.0;
        let max = vec![r, g, b].iter().cloned().fold(0.0/0.0, f32::max);
        let min = vec![r, g, b].iter().cloned().fold(0.0/0.0, f32::min);
        let mut h: f32 = 0.0;
        let mut s: f32 = 0.0;
        let l = ((max + min) / 2.0 * 100.0).round();

        if max != min {
            let delta = max - min;
            s = if l > 50.0 {
                ( delta / (2.0 - max - min) ) * 100.0
            } else {
                ( delta / (max + min) ) * 100.0
            };

            if max == r {
                h = (g - b) / delta + if g < b { 6.0 } else { 0.0 };
            } else if max == g {
                h = (b - r) / delta + 2.0
            } else if max == b {
                h = (r - g) / delta + 4.0
            }

            h = h * 60.0;

            h = h.round();
            s = s.round();
        }

        HSL { h, s, l }
    }
}

#[test]
fn test_compaction() {
    assert!(RGB::new(0xff, 0xff, 0xff).is_compactable());
    assert!(RGB::new(0xee, 0xee, 0xee).is_compactable());
    assert!(RGB::new(0x00, 0x00, 0x00).is_compactable());
    assert!(!RGB::new(0xf7, 0xf7, 0xf7).is_compactable());
    assert!(!RGB::new(0xff, 0xf7, 0xff).is_compactable());
}

#[test]
fn test_hsl() {
    let rgb_white = RGB::new(0xff, 0xff, 0xff);
    assert_eq!{HSL::from_rgb(rgb_white), HSL { h: 0.0, s: 0.0, l: 100.0 }};
    
    let rgb_red = RGB::new(0xff, 0, 0);
    assert_eq!{HSL::from_rgb(rgb_red), HSL { h: 0.0, s: 100.0, l: 50.0 }};

    let rgb_green = RGB::new(0, 0xff, 0);
    assert_eq!{HSL::from_rgb(rgb_green), HSL { h: 120.0, s: 100.0, l: 50.0 }};

    let rgb_blue = RGB::new(0, 0, 0xff);
    assert_eq!{HSL::from_rgb(rgb_blue), HSL { h: 240.0, s: 100.0, l: 50.0 }};

    let rgb_yellow = RGB::new(0xff, 0xff, 0);
    assert_eq!{HSL::from_rgb(rgb_yellow), HSL { h: 60.0, s: 100.0, l: 50.0 }};

    let rgb_cyan = RGB::new(14, 115, 123);
    assert_eq!{HSL::from_rgb(rgb_cyan), HSL { h: 184.0, s: 80.0, l: 27.0 }};
}
