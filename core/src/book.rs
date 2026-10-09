//! Thin safe wrapper around one EB_Book.

use std::ffi::CString;
use std::mem::MaybeUninit;
use std::os::raw::{c_char, c_int};
use std::path::Path;
use std::sync::Once;

use crate::ffi::*;
use crate::render::{self, Render};
use crate::{Error, Pos};

static INIT: Once = Once::new();

pub struct Book {
    raw: Box<EB_Book>,
    hookset: Box<EB_Hookset>,
}

// libeb keeps no thread-local state; callers serialize access with a Mutex.
unsafe impl Send for Book {}

fn check(code: EB_Error_Code) -> Result<(), Error> {
    if code == EB_SUCCESS as EB_Error_Code {
        Ok(())
    } else {
        Err(Error::Eb(code))
    }
}

fn euc_to_string(bytes: &[u8]) -> String {
    let end = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
    encoding_rs::EUC_JP.decode_without_bom_handling(&bytes[..end]).0.into_owned()
}

impl Book {
    pub fn open(path: &Path) -> Result<Book, Error> {
        INIT.call_once(|| unsafe {
            eb_initialize_library();
        });
        let cpath = CString::new(path.to_string_lossy().as_bytes()).map_err(|_| Error::Path)?;
        unsafe {
            let mut raw: Box<EB_Book> = Box::new(MaybeUninit::zeroed().assume_init());
            eb_initialize_book(&mut *raw);
            let mut hookset: Box<EB_Hookset> = Box::new(MaybeUninit::zeroed().assume_init());
            eb_initialize_hookset(&mut *hookset);
            render::install_hooks(&mut hookset);
            let mut book = Book { raw, hookset };
            check(eb_bind(&mut *book.raw, cpath.as_ptr()))?;
            Ok(book)
        }
    }

    pub fn subbooks(&mut self) -> Vec<EB_Subbook_Code> {
        let mut list = [0 as EB_Subbook_Code; EB_MAX_SUBBOOKS as usize];
        let mut count: c_int = 0;
        unsafe {
            if eb_subbook_list(&mut *self.raw, list.as_mut_ptr(), &mut count) != 0 {
                return vec![];
            }
        }
        list[..count as usize].to_vec()
    }

    pub fn character_code(&mut self) -> EB_Character_Code {
        let mut code = 0;
        unsafe { eb_character_code(&mut *self.raw, &mut code) };
        code
    }

    /// Selects a subbook and its smallest font.
    pub fn select(&mut self, subbook: EB_Subbook_Code) -> Result<(), Error> {
        unsafe {
            check(eb_set_subbook(&mut *self.raw, subbook))?;
            let mut fonts = [0 as EB_Font_Code; EB_MAX_FONTS as usize];
            let mut count = 0;
            if eb_font_list(&mut *self.raw, fonts.as_mut_ptr(), &mut count) == 0 && count > 0 {
                eb_set_font(&mut *self.raw, fonts[0]);
            }
        }
        Ok(())
    }

    pub fn title(&mut self) -> String {
        let mut buf = [0u8; EB_MAX_TITLE_LENGTH as usize + 1];
        unsafe { eb_subbook_title(&mut *self.raw, buf.as_mut_ptr() as *mut c_char) };
        euc_to_string(&buf)
    }

    pub fn has_word(&mut self) -> bool {
        unsafe { eb_have_word_search(&mut *self.raw) != 0 }
    }
    pub fn has_endword(&mut self) -> bool {
        unsafe { eb_have_endword_search(&mut *self.raw) != 0 }
    }
    pub fn has_exactword(&mut self) -> bool {
        unsafe { eb_have_exactword_search(&mut *self.raw) != 0 }
    }

    /// Runs a word search on the current subbook and returns all (heading, text) positions.
    pub fn search(
        &mut self,
        word: &[u8],
        f: unsafe extern "C" fn(*mut EB_Book, *const c_char) -> EB_Error_Code,
    ) -> Result<Vec<(Pos, Pos)>, Error> {
        let mut word = word.to_vec();
        word.push(0);
        unsafe {
            if f(&mut *self.raw, word.as_ptr() as *const c_char) != 0 {
                // Invalid or empty word for this book: nothing matches.
                return Ok(vec![]);
            }
            let mut hits = vec![MaybeUninit::<EB_Hit>::zeroed().assume_init(); 256];
            let mut out = vec![];
            loop {
                let mut count = 0;
                check(eb_hit_list(&mut *self.raw, hits.len() as c_int, hits.as_mut_ptr(), &mut count))?;
                if count == 0 {
                    break;
                }
                for h in &hits[..count as usize] {
                    out.push((Pos::from(h.heading), Pos::from(h.text)));
                }
            }
            Ok(out)
        }
    }

    pub fn heading(&mut self, pos: Pos, r: &mut Render) -> Result<String, Error> {
        unsafe {
            check(eb_seek_text(&mut *self.raw, &pos.into()))?;
        }
        r.heading = true;
        let html = self.read_with(r, eb_read_heading);
        r.heading = false;
        html
    }

    pub fn seek(&mut self, pos: Pos) -> Result<(), Error> {
        unsafe { check(eb_seek_text(&mut *self.raw, &pos.into())) }
    }

    pub fn tell(&mut self) -> Result<Pos, Error> {
        let mut p = EB_Position { page: 0, offset: 0 };
        unsafe { check(eb_tell_text(&mut *self.raw, &mut p))? };
        Ok(p.into())
    }

    /// Reads the text from the current position up to the next stop code.
    pub fn read_entry(&mut self, r: &mut Render) -> Result<String, Error> {
        self.read_with(r, eb_read_text)
    }

    /// Moves to the start of the next entry. Returns false at the end of the text.
    pub fn forward(&mut self) -> Result<bool, Error> {
        match unsafe { eb_forward_text(&mut *self.raw, std::ptr::null_mut()) } {
            0 => Ok(true),
            c if c == EB_ERR_END_OF_CONTENT as c_int => Ok(false),
            c => Err(Error::Eb(c)),
        }
    }

    /// Moves to the start of the previous entry. Returns false at the start of the text.
    pub fn backward(&mut self) -> Result<bool, Error> {
        match unsafe { eb_backward_text(&mut *self.raw, std::ptr::null_mut()) } {
            0 => Ok(true),
            c if c == EB_ERR_END_OF_CONTENT as c_int => Ok(false),
            c => Err(Error::Eb(c)),
        }
    }

    fn read_with(
        &mut self,
        r: &mut Render,
        f: unsafe extern "C" fn(
            *mut EB_Book,
            *mut EB_Appendix,
            *mut EB_Hookset,
            *mut std::ffi::c_void,
            usize,
            *mut c_char,
            *mut isize,
        ) -> EB_Error_Code,
    ) -> Result<String, Error> {
        let mut out = Vec::new();
        let mut buf = [0u8; 4096];
        loop {
            let mut len: isize = 0;
            unsafe {
                check(f(
                    &mut *self.raw,
                    std::ptr::null_mut(),
                    &mut *self.hookset,
                    r as *mut Render as *mut std::ffi::c_void,
                    buf.len(),
                    buf.as_mut_ptr() as *mut c_char,
                    &mut len,
                ))?;
            }
            out.extend_from_slice(&buf[..len as usize]);
            if len == 0 || unsafe { eb_is_text_stopped(&mut *self.raw) } != 0 {
                break;
            }
        }
        Ok(r.finish(String::from_utf8_lossy(&out).into_owned()))
    }
}

impl Drop for Book {
    fn drop(&mut self) {
        unsafe {
            eb_finalize_hookset(&mut *self.hookset);
            eb_finalize_book(&mut *self.raw);
        }
    }
}

impl From<EB_Position> for Pos {
    fn from(p: EB_Position) -> Pos {
        Pos { page: p.page, offset: p.offset }
    }
}

impl From<Pos> for EB_Position {
    fn from(p: Pos) -> EB_Position {
        EB_Position { page: p.page, offset: p.offset }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn binds_book_and_lists_subbooks() {
        let root = crate::test_root();
        let mut book = Book::open(&root.join("S-I-Dic97")).unwrap();
        assert_eq!(book.subbooks().len(), 10);
    }
}
