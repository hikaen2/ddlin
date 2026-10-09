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

/// Entries read per full-text search call.
const FULL_TEXT_CHUNK: usize = 2000;

/// Approximate size of one text segment in bytes of text, not counting
/// markup and gaiji images (DDwin's BufferSize).
const SEGMENT_SIZE: usize = 8000;

#[derive(Debug)]
pub enum Error {
    Eb(i32),
    Path,
    NoDict,
    NoText,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Error::Eb(code) => write!(f, "EB error {code}"),
            Error::Path => write!(f, "invalid path"),
            Error::NoDict => write!(f, "no such dictionary"),
            Error::NoText => write!(f, "no text"),
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
    pub full: bool,
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

/// Part of a full-text search. `next` is where to continue, or None when
/// the end of the text is reached.
#[derive(Debug, Clone, Serialize)]
pub struct FullText {
    pub hits: Vec<Hit>,
    pub next: Option<Pos>,
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
                let full = book.text_start().is_some();
                entries.push((sb, book.title(), book.has_word(), book.has_endword(), book.has_exactword(), menu, full));
            }
            for (sb, title, prefix, suffix, exact, menu, full) in entries {
                dicts.push(Dict { id: dicts.len(), title, prefix, suffix, exact, menu, full });
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

    /// Returns every heading that matches, in the order the entries are
    /// stored in the text, without duplicates of the same text position.
    pub fn search(&self, id: usize, query: &str, mode: Mode) -> Result<Vec<Hit>, Error> {
        self.with(id, |book, cache| {
            let word = encode_query(book, query);
            let f = match mode {
                Mode::Prefix => ffi::eb_search_word,
                Mode::Suffix => ffi::eb_search_endword,
                Mode::Exact => ffi::eb_search_exactword,
            };
            // Storage order: by position in the text, as DDwin's 格納順.
            let mut found = book.search(&word, f)?;
            found.sort_by_key(|&(_, text)| (text.page, text.offset));
            found.dedup_by_key(|&mut (_, text)| text);
            let mut r = Render::new(cache);
            let hits = found
                .into_iter()
                .map(|(heading, text)| Ok(Hit { heading: book.heading(heading, &mut r)?, pos: text }))
                .collect::<Result<Vec<_>, Error>>()?;
            Ok(match kana_key(query) {
                Some(key) => hits.into_iter().filter(|h| reading_matches(&h.heading, &key, mode)).collect(),
                None => hits,
            })
        })
    }

    /// Searches the text of up to FULL_TEXT_CHUNK entries for `query`,
    /// starting at `from` or at the start of the text. Matching ignores
    /// markup and ASCII case. Each hit's heading is the entry's first line.
    pub fn full_text(&self, id: usize, query: &str, from: Option<Pos>) -> Result<FullText, Error> {
        let needle = query.to_lowercase();
        if needle.is_empty() {
            return Ok(FullText { hits: vec![], next: None });
        }
        self.with(id, |book, cache| {
            let mut cur = match from {
                Some(p) => p,
                None => book.text_start().ok_or(Error::NoText)?,
            };
            let mut hits = vec![];
            for _ in 0..FULL_TEXT_CHUNK {
                book.seek(cur)?;
                let mut r = Render::new(cache);
                r.plain = true;
                let text = plain_text(&book.read_entry(&mut r)?);
                if text.to_lowercase().contains(&needle) {
                    book.seek(cur)?;
                    let html = book.read_entry(&mut Render::new(cache))?;
                    hits.push(Hit { heading: first_line(&html).to_string(), pos: cur });
                }
                if !book.forward()? {
                    return Ok(FullText { hits, next: None });
                }
                cur = book.tell()?;
            }
            Ok(FullText { hits, next: Some(cur) })
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

/// `s` with katakana as hiragana and without ‐・ー and spaces.
fn normalize_kana(s: &str) -> String {
    s.chars()
        .filter(|c| !matches!(c, '‐' | '・' | 'ー' | '-' | ' ' | '\u{3000}'))
        .map(|c| match c {
            'ァ'..='ヶ' => char::from_u32(c as u32 - 0x60).unwrap(),
            _ => c,
        })
        .collect()
}

fn is_kana(c: char) -> bool {
    ('ぁ'..='ゖ').contains(&c)
}

/// The query as normalized kana, or None if it is not all kana.
fn kana_key(query: &str) -> Option<String> {
    let key = normalize_kana(query);
    (!key.is_empty() && key.chars().all(is_kana)).then_some(key)
}

/// Kana indexes ignore voicing marks for many entries, so "らが" also
/// finds "らか". Compare the heading (before 【) with the query from the
/// matched end, as DDwin does: kana must be equal, and a non-kana
/// character such as a kanji ends the comparison as a match.
fn reading_matches(heading: &str, key: &str, mode: Mode) -> bool {
    let text = plain_text(heading);
    let reading = normalize_kana(text.split('【').next().unwrap_or(""));
    let agree = |(h, k): (char, char)| if is_kana(h) { (h == k).then_some(true) } else { Some(false) };
    let first_difference = match mode {
        Mode::Prefix => reading.chars().zip(key.chars()).map(agree).find(|r| r != &Some(true)),
        Mode::Suffix => reading.chars().rev().zip(key.chars().rev()).map(agree).find(|r| r != &Some(true)),
        Mode::Exact => return !reading.chars().all(is_kana) || reading == key,
    };
    // None: all compared kana are equal; Some(Some(false)): stopped at a
    // non-kana character; Some(None): a kana differs.
    first_difference != Some(None)
}

/// The text outside tags, with line breaks as newlines.
fn plain_text(html: &str) -> String {
    let html = html.replace("<br>", "\n");
    let mut out = String::with_capacity(html.len());
    let mut in_tag = false;
    for c in html.chars() {
        match c {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => out.push(c),
            _ => {}
        }
    }
    out.replace("&lt;", "<").replace("&gt;", ">").replace("&amp;", "&")
}

/// The first line of an entry's HTML.
fn first_line(html: &str) -> &str {
    html.split("<br>").next().unwrap_or(html)
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
