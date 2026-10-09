//! Tests against the real dictionaries in $DDLIN_TEST_ROOT (default ~/opt/EPWING).

use std::path::PathBuf;
use std::sync::OnceLock;

use ddlin_core::{Dir, Library, Mode, Pos};

fn lib() -> &'static Library {
    static LIB: OnceLock<Library> = OnceLock::new();
    LIB.get_or_init(|| Library::open(&[root()]))
}

fn root() -> PathBuf {
    std::env::var_os("DDLIN_TEST_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(std::env::var_os("HOME").unwrap()).join("opt/EPWING"))
}

fn dict(title: &str) -> usize {
    lib().dicts().iter().find(|d| d.title == title).unwrap().id
}

const KOJIEN: &str = "広辞苑　第四版";

fn headings(title: &str, query: &str, mode: Mode) -> Vec<String> {
    lib().search(dict(title), query, mode).unwrap().into_iter().map(|h| h.heading).collect()
}

fn pos_of(title: &str, query: &str) -> Pos {
    lib().search(dict(title), query, Mode::Exact).unwrap()[0].pos
}

#[test]
fn lists_all_subbooks_and_skips_non_books() {
    // EBX1100: 8, S-I-Dic97: 10, epwing: 1; dd266 is not a book.
    assert_eq!(lib().dicts().len(), 19);
}

#[test]
fn opens_book_folders_in_order_once() {
    let r = root();
    let lib = Library::open(&[r.join("S-I-Dic97"), r.join("EBX1100"), r.join("S-I-Dic97"), r.clone()]);
    // S-I-Dic97 (10), EBX1100 (8), then the rest of the parent folder (epwing: 1).
    assert_eq!(lib.dicts().len(), 19);
    assert_eq!(lib.dicts()[0].title, "広辞苑　第四版");
    assert_eq!(lib.dicts()[10].title, "三省堂のしゃべる英和辞典");
}

#[test]
fn titles_are_utf8() {
    let titles: Vec<_> = lib().dicts().iter().map(|d| d.title.as_str()).collect();
    assert!(titles.contains(&KOJIEN));
    assert!(titles.contains(&"三省堂のしゃべる英和辞典"));
}

#[test]
fn reports_search_methods() {
    let d = &lib().dicts()[dict(KOJIEN)];
    assert!(d.prefix && d.suffix && d.exact);
    let d = &lib().dicts()[dict("付属資料")];
    assert!(!d.prefix && !d.suffix && !d.exact);
}

#[test]
fn prefix_search_finds_longer_words() {
    let h = headings(KOJIEN, "どうぶ", Mode::Prefix);
    assert!(h.iter().any(|s| s.contains("童舞")));
    assert!(h.iter().any(|s| s.contains("【<span class=\"kw\">動物</span>】")));
}

#[test]
fn exact_search_excludes_longer_words() {
    let h = headings(KOJIEN, "どうぶつ", Mode::Exact);
    assert!(h.iter().any(|s| s.contains(">動物<")));
    assert!(!h.iter().any(|s| s.contains("動物園")));
}

#[test]
fn suffix_search() {
    let h = headings(KOJIEN, "どうぶつ", Mode::Suffix);
    assert!(h.iter().any(|s| s.contains(">動物<")));
}

#[test]
fn ebxa_dictionary_search() {
    let h = headings("三省堂のしゃべる英和辞典", "dog", Mode::Prefix);
    assert!(h.iter().any(|s| s.starts_with("dog")));
}

#[test]
fn short_prefix_returns_everything_once() {
    let hits = lib().search(dict(KOJIEN), "あ", Mode::Prefix).unwrap();
    assert!(hits.len() > 7000);
    assert!(hits.last().unwrap().heading.contains("あんわ"));
    let unique: std::collections::HashSet<_> = hits.iter().map(|h| h.pos).collect();
    assert_eq!(unique.len(), hits.len());
}

#[test]
fn no_match_is_empty() {
    assert!(headings(KOJIEN, "ぢぢぢぢぢ", Mode::Prefix).is_empty());
}

#[test]
fn entry_html() {
    let seg = lib().read_segment(dict(KOJIEN), pos_of(KOJIEN, "どうぶつ"), Dir::Forward).unwrap();
    let first = &seg.html[..seg.html.find("</div>").unwrap()];
    assert!(first.starts_with("<div class=\"entry\"><span class=\"kw\">どう‐ぶつ</span>【<span class=\"kw\">動物</span>】<br>"));
    assert!(first.contains("<img class=\"gaiji\" src=\"data:image/png;base64,"));
}

#[test]
fn references_are_links() {
    let seg = lib().read_segment(dict(KOJIEN), pos_of(KOJIEN, "どうふけん"), Dir::Forward).unwrap();
    let first = &seg.html[..seg.html.find("</div>").unwrap()];
    let i = first.find("<a data-pos=\"").expect("link");
    let rest = &first[i + 13..];
    let (page, offset) = rest[..rest.find('"').unwrap()].split_once(':').unwrap();
    let target = Pos { page: page.parse().unwrap(), offset: offset.parse().unwrap() };
    let seg = lib().read_segment(dict(KOJIEN), target, Dir::Forward).unwrap();
    assert!(seg.html.starts_with("<div class=\"entry\"><span class=\"kw\">と‐どう‐ふ‐けん</span>"), "{}", &seg.html[..200]);
}

#[test]
fn forward_segments_are_sized_and_continue() {
    let id = dict(KOJIEN);
    let a = lib().read_segment(id, pos_of(KOJIEN, "どうぶつ"), Dir::Forward).unwrap();
    assert!(a.html.len() >= 8000);
    assert!(a.html.matches("<div class=\"entry\">").count() > 5);
    assert!(a.html.ends_with("</div>"));
    let b = lib().read_segment(id, a.next.unwrap(), Dir::Forward).unwrap();
    assert!(b.html.starts_with("<div class=\"entry\">"));
    assert_ne!(a.html, b.html);
}

#[test]
fn backward_joins_forward_without_gap() {
    let id = dict(KOJIEN);
    let start = pos_of(KOJIEN, "どうぶつ");
    let a = lib().read_segment(id, start, Dir::Forward).unwrap();
    let b = lib().read_segment(id, a.next.unwrap(), Dir::Forward).unwrap();
    // Reading backward from b's start must end with exactly a's last entries.
    let back = lib().read_segment(id, a.next.unwrap(), Dir::Backward).unwrap();
    let last_entry = &a.html[a.html.rfind("<div class=\"entry\">").unwrap()..];
    assert!(back.html.ends_with(last_entry));
    // And continuing backward from there reaches the entry before `start`.
    let before = lib().read_segment(id, start, Dir::Backward).unwrap();
    assert!(before.html.ends_with("</div>"));
    assert!(!before.html.contains("【<span class=\"kw\">動物</span>】<br>"));
    let _ = b;
}

#[test]
fn edges_of_dictionary() {
    let id = dict(KOJIEN);
    let first = lib().search(id, "あ", Mode::Exact).unwrap()[0].pos;
    let back = lib().read_segment(id, first, Dir::Backward).unwrap();
    assert!(back.next.is_none());
    assert!(back.html.is_empty());

    let last = lib().search(id, "ん", Mode::Prefix).unwrap();
    let last = last.iter().map(|h| h.pos).max_by_key(|p| (p.page, p.offset)).unwrap();
    let mut seg = lib().read_segment(id, last, Dir::Forward).unwrap();
    for _ in 0..1000 {
        match seg.next {
            Some(p) => seg = lib().read_segment(id, p, Dir::Forward).unwrap(),
            None => break,
        }
    }
    assert!(seg.next.is_none());
}

#[test]
fn ebxa_japanese_query() {
    let h = headings("しゃべるクラウン和英辞典", "いぬ", Mode::Exact);
    assert!(h.iter().any(|s| s.contains("犬")), "{h:?}");
}

#[test]
fn menu_items_are_links() {
    let id = dict("付属資料");
    assert!(lib().dicts()[id].menu);
    let pos = lib().menu(id).unwrap().unwrap();
    let seg = lib().read_segment(id, pos, Dir::Forward).unwrap();
    assert!(seg.html.contains("\">１．凡例</a>"), "{}", seg.html);
    assert!(!lib().dicts()[dict("取扱説明")].menu);
}

#[test]
fn graphics_are_images() {
    let id = dict(KOJIEN);
    // A colour photo (吉野源三郎) and a monochrome drawing (アオジ) from the menu.
    let photo = lib().read_segment(id, Pos { page: 17751, offset: 818 }, Dir::Forward).unwrap();
    assert!(photo.html.contains("<img class=\"graphic\" src=\"data:image/bmp;base64,Qk"));
    let drawing = lib().read_segment(id, Pos { page: 17682, offset: 1682 }, Dir::Forward).unwrap();
    assert!(drawing.html.contains("アオジ<img class=\"graphic\" src=\"data:image/bmp;base64,Qk"));
}

#[test]
fn headings_have_no_graphics() {
    // 漢字源 headings carry a large glyph graphic; only the text pane shows it.
    let hits = lib().search(dict("漢字源"), "いぬ", Mode::Prefix).unwrap();
    let inu = hits.iter().find(|h| h.heading.contains("【犬】")).unwrap();
    assert!(!inu.heading.contains("<img"), "{}", inu.heading);
    let seg = lib().read_segment(dict("漢字源"), inu.pos, Dir::Forward).unwrap();
    assert!(seg.html.contains("<img class=\"graphic\""));
}
