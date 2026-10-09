mod book;
mod ffi;
mod render;

// libeb needs zlib at link time.
use libz_sys as _;

use std::path::PathBuf;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};

use book::Book;
use render::{GaijiCache, Render};

/// Approximate size of one text segment in bytes of text, not counting
/// markup and gaiji images (DDwin's BufferSize).
const SEGMENT_SIZE: usize = 8000;

#[derive(Debug)]
pub enum Error {
    Eb(i32),
    Path,
    NoDict,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Error::Eb(code) => write!(f, "EB error {code}"),
            Error::Path => write!(f, "invalid path"),
            Error::NoDict => write!(f, "no such dictionary"),
        }
    }
}

impl std::error::Error for Error {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Pos {
    pub page: i32,
    pub offset: i32,
}

#[derive(Debug, Clone, Serialize)]
pub struct Dict {
    pub id: usize,
    pub title: String,
    pub prefix: bool,
    pub suffix: bool,
    pub exact: bool,
    pub menu: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum Mode {
    Prefix,
    Suffix,
    Exact,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum Dir {
    Forward,
    Backward,
}

#[derive(Debug, Clone, Serialize)]
pub struct Hit {
    pub heading: String,
    pub pos: Pos,
}

/// A run of entries. `next` is where to continue reading in the same
/// direction, or None at the start/end of the dictionary.
#[derive(Debug, Clone, Serialize)]
pub struct Segment {
    pub html: String,
    pub next: Option<Pos>,
}

struct Slot {
    book: usize,
    subbook: i32,
    gaiji: GaijiCache,
}

/// libeb keeps global state (e.g. its text cache), so every call into it,
/// from any Library, goes through this lock.
static EB: Mutex<()> = Mutex::new(());

fn eb_lock() -> std::sync::MutexGuard<'static, ()> {
    EB.lock().unwrap_or_else(|e| e.into_inner())
}

struct Books {
    books: Vec<Book>,
    slots: Vec<Slot>,
}

pub struct Library {
    dicts: Vec<Dict>,
    inner: Mutex<Books>,
}

impl Library {
    /// Opens the books in `folders`, in order. A folder that is not a book
    /// itself is searched for books directly under it; other directories
    /// are skipped. The same book is opened only once.
    pub fn open(folders: &[PathBuf]) -> Library {
        let _eb = eb_lock();
        let mut dirs = vec![];
        for folder in folders {
            if Book::open(folder).is_ok() {
                dirs.push(folder.clone());
                continue;
            }
            let mut subdirs: Vec<_> = std::fs::read_dir(folder)
                .map(|rd| rd.flatten().map(|e| e.path()).filter(|p| p.is_dir()).collect())
                .unwrap_or_default();
            subdirs.sort();
            dirs.extend(subdirs);
        }
        let mut seen = std::collections::HashSet::new();
        dirs.retain(|d| seen.insert(d.canonicalize().unwrap_or_else(|_| d.clone())));

        let mut dicts = vec![];
        let mut inner = Books { books: vec![], slots: vec![] };
        for dir in dirs {
            let Ok(mut book) = Book::open(&dir) else { continue };
            let subbooks = book.subbooks();
            let mut entries = vec![];
            for &sb in &subbooks {
                if book.select(sb).is_err() {
                    continue;
                }
                let menu = book.menu().is_some();
                entries.push((sb, book.title(), book.has_word(), book.has_endword(), book.has_exactword(), menu));
            }
            for (sb, title, prefix, suffix, exact, menu) in entries {
                dicts.push(Dict { id: dicts.len(), title, prefix, suffix, exact, menu });
                inner.slots.push(Slot { book: inner.books.len(), subbook: sb, gaiji: GaijiCache::new() });
            }
            inner.books.push(book);
        }
        Library { dicts, inner: Mutex::new(inner) }
    }

    pub fn dicts(&self) -> &[Dict] {
        &self.dicts
    }

    fn with<T>(&self, id: usize, f: impl FnOnce(&mut Book, &mut GaijiCache) -> Result<T, Error>) -> Result<T, Error> {
        let _eb = eb_lock();
        let mut guard = self.inner.lock().unwrap();
        let Books { books, slots } = &mut *guard;
        let slot = slots.get_mut(id).ok_or(Error::NoDict)?;
        let book = &mut books[slot.book];
        book.select(slot.subbook)?;
        f(book, &mut slot.gaiji)
    }

    /// Returns every heading that matches, without duplicates of the same text position.
    pub fn search(&self, id: usize, query: &str, mode: Mode) -> Result<Vec<Hit>, Error> {
        self.with(id, |book, cache| {
            let word = encode_query(book, query);
            let f = match mode {
                Mode::Prefix => ffi::eb_search_word,
                Mode::Suffix => ffi::eb_search_endword,
                Mode::Exact => ffi::eb_search_exactword,
            };
            let mut seen = std::collections::HashSet::new();
            let mut hits = vec![];
            let mut r = Render::new(cache);
            for (heading, text) in book.search(&word, f)? {
                if seen.insert(text) {
                    hits.push(Hit { heading: book.heading(heading, &mut r)?, pos: text });
                }
            }
            Ok(hits)
        })
    }

    /// Start of the dictionary's menu, if it has one.
    pub fn menu(&self, id: usize) -> Result<Option<Pos>, Error> {
        self.with(id, |book, _| Ok(book.menu()))
    }

    /// Reads about SEGMENT_SIZE bytes of whole entries.
    ///
    /// Forward reads starting at `pos`; Backward reads the entries just
    /// before the entry starting at `pos`.
    pub fn read_segment(&self, id: usize, pos: Pos, dir: Dir) -> Result<Segment, Error> {
        self.with(id, |book, cache| {
            let mut r = Render::new(cache);
            match dir {
                Dir::Forward => read_forward(book, &mut r, pos),
                Dir::Backward => read_backward(book, &mut r, pos),
            }
        })
    }
}

impl Drop for Library {
    fn drop(&mut self) {
        let _eb = eb_lock();
        if let Ok(inner) = self.inner.get_mut() {
            inner.books.clear();
        }
    }
}

fn entry_html(book: &mut Book, r: &mut Render, pos: Pos) -> Result<String, Error> {
    book.seek(pos)?;
    Ok(format!("<div class=\"entry\">{}</div>", book.read_entry(r)?))
}

/// Length of the text outside tags.
fn text_len(html: &str) -> usize {
    let mut in_tag = false;
    let mut n = 0;
    for b in html.bytes() {
        match b {
            b'<' => in_tag = true,
            b'>' => in_tag = false,
            _ if !in_tag => n += 1,
            _ => {}
        }
    }
    n
}

fn read_forward(book: &mut Book, r: &mut Render, pos: Pos) -> Result<Segment, Error> {
    let mut html = String::new();
    let mut size = 0;
    let mut cur = pos;
    loop {
        let entry = entry_html(book, r, cur)?;
        size += text_len(&entry);
        html.push_str(&entry);
        if !book.forward()? {
            return Ok(Segment { html, next: None });
        }
        cur = book.tell()?;
        if size >= SEGMENT_SIZE {
            return Ok(Segment { html, next: Some(cur) });
        }
    }
}

fn read_backward(book: &mut Book, r: &mut Render, pos: Pos) -> Result<Segment, Error> {
    let mut parts = vec![];
    let mut size = 0;
    let mut cur = pos;
    let next = loop {
        book.seek(cur)?;
        // libeb may fail near the start of the text; treat it as the start.
        if !book.backward().unwrap_or(false) {
            break None;
        }
        cur = book.tell()?;
        let entry = entry_html(book, r, cur)?;
        size += text_len(&entry);
        parts.push(entry);
        if size >= SEGMENT_SIZE {
            break Some(cur);
        }
    };
    parts.reverse();
    Ok(Segment { html: parts.concat(), next })
}

fn encode_query(book: &mut Book, query: &str) -> Vec<u8> {
    if book.character_code() == ffi::EB_CHARCODE_ISO8859_1 as i32 {
        encoding_rs::WINDOWS_1252.encode(query).0.into_owned()
    } else {
        encoding_rs::EUC_JP.encode(query).0.into_owned()
    }
}

/// Dictionary folder used by tests: $DDLIN_TEST_ROOT or ~/opt/EPWING.
#[cfg(test)]
pub(crate) fn test_root() -> std::path::PathBuf {
    std::env::var_os("DDLIN_TEST_ROOT")
        .map(Into::into)
        .unwrap_or_else(|| std::path::PathBuf::from(std::env::var_os("HOME").unwrap()).join("opt/EPWING"))
}
