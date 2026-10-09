//! libeb hooks that turn dictionary text into HTML.

use std::collections::HashMap;
use std::os::raw::{c_char, c_int, c_uint, c_void};

use base64::Engine;

use crate::ffi::*;
use crate::Pos;

/// Gaiji images keyed by (wide, code).
pub type GaijiCache = HashMap<(bool, u32), String>;

pub struct Render<'a> {
    pub gaiji: &'a mut GaijiCache,
    pub heading: bool,
    refs: Vec<Option<Pos>>,
}

impl<'a> Render<'a> {
    pub fn new(gaiji: &'a mut GaijiCache) -> Self {
        Render { gaiji, heading: false, refs: vec![] }
    }

    /// Resolves reference placeholders written by the hooks.
    pub fn finish(&mut self, html: String) -> String {
        if self.refs.is_empty() {
            return html;
        }
        let mut out = String::with_capacity(html.len());
        let mut rest = html.as_str();
        while let Some(i) = rest.find("<a data-ref=\"") {
            out.push_str(&rest[..i]);
            rest = &rest[i + 13..];
            let end = rest.find('"').unwrap();
            let n: usize = rest[..end].parse().unwrap();
            rest = &rest[end + 2..];
            match self.refs.get(n).copied().flatten() {
                Some(p) => out.push_str(&format!("<a data-pos=\"{}:{}\">", p.page, p.offset)),
                None => out.push_str("<a>"),
            }
        }
        out.push_str(rest);
        self.refs.clear();
        out
    }
}

pub fn install_hooks(hookset: &mut EB_Hookset) {
    let hooks: &[(u32, Hook)] = &[
        (EB_HOOK_NARROW_JISX0208, char_hook),
        (EB_HOOK_WIDE_JISX0208, char_hook),
        (EB_HOOK_GB2312, char_hook),
        (EB_HOOK_ISO8859_1, char_hook),
        (EB_HOOK_NARROW_FONT, gaiji_hook),
        (EB_HOOK_WIDE_FONT, gaiji_hook),
        (EB_HOOK_EBXAC_GAIJI, gaiji_hook),
        (EB_HOOK_NEWLINE, tag_hook),
        (EB_HOOK_BEGIN_EMPHASIS, tag_hook),
        (EB_HOOK_END_EMPHASIS, tag_hook),
        (EB_HOOK_BEGIN_SUBSCRIPT, tag_hook),
        (EB_HOOK_END_SUBSCRIPT, tag_hook),
        (EB_HOOK_BEGIN_SUPERSCRIPT, tag_hook),
        (EB_HOOK_END_SUPERSCRIPT, tag_hook),
        (EB_HOOK_BEGIN_KEYWORD, tag_hook),
        (EB_HOOK_END_KEYWORD, tag_hook),
        (EB_HOOK_BEGIN_REFERENCE, tag_hook),
        (EB_HOOK_END_REFERENCE, tag_hook),
    ];
    for &(code, f) in hooks {
        let hook = EB_Hook { code: code as c_int, function: Some(f) };
        unsafe { eb_set_hook(hookset, &hook) };
    }
}

type Hook = unsafe extern "C" fn(
    *mut EB_Book,
    *mut EB_Appendix,
    *mut c_void,
    EB_Hook_Code,
    c_int,
    *const c_uint,
) -> EB_Error_Code;

unsafe fn write(book: *mut EB_Book, s: &str) {
    eb_write_text(book, s.as_ptr() as *const c_char, s.len());
}

/// Decodes one character passed to a character hook.
pub fn decode_char(hook: u32, code: u32) -> String {
    let pair = [(code >> 8) as u8, code as u8];
    let s = match hook {
        EB_HOOK_ISO8859_1 => return (code as u8 as char).to_string(),
        EB_HOOK_GB2312 => encoding_rs::GBK.decode_without_bom_handling(&pair).0,
        _ => encoding_rs::EUC_JP.decode_without_bom_handling(&pair).0,
    };
    if hook != EB_HOOK_NARROW_JISX0208 {
        return s.into_owned();
    }
    // Narrow JIS X 0208: show full-width ASCII as half-width.
    s.chars()
        .map(|c| match c {
            '\u{3000}' => ' ',
            '\u{FF01}'..='\u{FF5E}' => char::from_u32(c as u32 - 0xFEE0).unwrap(),
            _ => c,
        })
        .collect()
}

pub fn escape(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

unsafe extern "C" fn char_hook(
    book: *mut EB_Book,
    _: *mut EB_Appendix,
    _: *mut c_void,
    hook: EB_Hook_Code,
    _: c_int,
    argv: *const c_uint,
) -> EB_Error_Code {
    write(book, &escape(&decode_char(hook as u32, *argv)));
    0
}

unsafe extern "C" fn tag_hook(
    book: *mut EB_Book,
    _: *mut EB_Appendix,
    container: *mut c_void,
    hook: EB_Hook_Code,
    _: c_int,
    argv: *const c_uint,
) -> EB_Error_Code {
    let r = &mut *(container as *mut Render);
    let tag = match hook as u32 {
        EB_HOOK_NEWLINE if r.heading => "",
        EB_HOOK_NEWLINE => "<br>",
        EB_HOOK_BEGIN_EMPHASIS => "<b>",
        EB_HOOK_END_EMPHASIS => "</b>",
        EB_HOOK_BEGIN_SUBSCRIPT => "<sub>",
        EB_HOOK_END_SUBSCRIPT => "</sub>",
        EB_HOOK_BEGIN_SUPERSCRIPT => "<sup>",
        EB_HOOK_END_SUPERSCRIPT => "</sup>",
        EB_HOOK_BEGIN_KEYWORD => "<span class=\"kw\">",
        EB_HOOK_END_KEYWORD => "</span>",
        EB_HOOK_BEGIN_REFERENCE => {
            r.refs.push(None);
            write(book, &format!("<a data-ref=\"{}\">", r.refs.len() - 1));
            return 0;
        }
        EB_HOOK_END_REFERENCE => {
            if let Some(last) = r.refs.last_mut() {
                *last = Some(Pos { page: *argv.add(1) as i32, offset: *argv.add(2) as i32 });
            }
            "</a>"
        }
        _ => "",
    };
    write(book, tag);
    0
}

unsafe extern "C" fn gaiji_hook(
    book: *mut EB_Book,
    _: *mut EB_Appendix,
    container: *mut c_void,
    hook: EB_Hook_Code,
    _: c_int,
    argv: *const c_uint,
) -> EB_Error_Code {
    let r = &mut *(container as *mut Render);
    let wide = hook as u32 != EB_HOOK_NARROW_FONT;
    let code = *argv;
    let img = r
        .gaiji
        .entry((wide, code))
        .or_insert_with(|| gaiji_img(book, wide, code).unwrap_or_else(|| "\u{3013}".into()))
        .clone();
    write(book, &img);
    0
}

/// Returns an <img> tag for a gaiji, or None if the book has no bitmap for it.
unsafe fn gaiji_img(book: *mut EB_Book, wide: bool, code: u32) -> Option<String> {
    let mut width = 0;
    let mut height = 0;
    let mut bitmap = [0 as c_char; 512];
    let ok = if wide {
        eb_wide_font_width(book, &mut width) == 0
            && eb_wide_font_character_bitmap(book, code as c_int, bitmap.as_mut_ptr()) == 0
    } else {
        eb_narrow_font_width(book, &mut width) == 0
            && eb_narrow_font_character_bitmap(book, code as c_int, bitmap.as_mut_ptr()) == 0
    };
    if !ok || eb_font_height(book, &mut height) != 0 {
        return None;
    }
    let mut png = [0 as c_char; 4096];
    let mut len = 0usize;
    if eb_bitmap_to_png(bitmap.as_ptr(), width, height, png.as_mut_ptr(), &mut len) != 0 {
        return None;
    }
    let bytes = std::slice::from_raw_parts(png.as_ptr() as *const u8, len);
    Some(format!(
        "<img class=\"gaiji\" src=\"data:image/png;base64,{}\">",
        base64::engine::general_purpose::STANDARD.encode(bytes)
    ))
}
