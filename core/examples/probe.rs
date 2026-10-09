//! Debug CLI: probe <root> [<dict-title-substring> <prefix|suffix|exact|pos|menu|full> <query|page:offset>]
use ddlin_core::{Dir, Library, Mode, Pos};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let lib = Library::open(&[args[0].clone().into()]);
    if args.len() < 4 {
        for d in lib.dicts() {
            println!("{} {} prefix={} suffix={} exact={}", d.id, d.title, d.prefix, d.suffix, d.exact);
        }
        return;
    }
    let dict = lib.dicts().iter().find(|d| d.title.contains(&args[1])).expect("dict");
    if args[2] == "full" {
        let t = std::time::Instant::now();
        let (mut from, mut calls, mut hits) = (None, 0, vec![]);
        loop {
            let chunk = lib.full_text(dict.id, &args[3], from).unwrap();
            calls += 1;
            hits.extend(chunk.hits);
            match chunk.next {
                Some(p) => from = Some(p),
                None => break,
            }
        }
        eprintln!("{} hits, {calls} calls in {:?}", hits.len(), t.elapsed());
        eprintln!("first={:?} last={:?}", hits.first().map(|h| h.pos), hits.last().map(|h| h.pos));
        for h in hits.iter().take(10) {
            println!("{:?} {}", h.pos, h.heading);
        }
        return;
    }
    if args[2] == "menu" {
        let pos = lib.menu(dict.id).unwrap().expect("no menu");
        let seg = lib.read_segment(dict.id, pos, Dir::Forward).unwrap();
        println!("menu={pos:?} next={:?}\n{}", seg.next, seg.html);
        return;
    }
    if args[2] == "pos" {
        let (page, offset) = args[3].split_once(':').unwrap();
        let pos = Pos { page: page.parse().unwrap(), offset: offset.parse().unwrap() };
        let seg = lib.read_segment(dict.id, pos, Dir::Forward).unwrap();
        println!("next={:?}\n{}", seg.next, seg.html);
        return;
    }
    let mode = match args[2].as_str() {
        "suffix" => Mode::Suffix,
        "exact" => Mode::Exact,
        _ => Mode::Prefix,
    };
    let t = std::time::Instant::now();
    let hits = lib.search(dict.id, &args[3], mode).unwrap();
    eprintln!("{} hits in {:?}", hits.len(), t.elapsed());
    for h in hits.iter().take(20) {
        println!("{:?} {}", h.pos, h.heading);
    }
    if let Some(h) = hits.first() {
        let seg = lib.read_segment(dict.id, h.pos, Dir::Forward).unwrap();
        println!("--- forward next={:?}\n{}", seg.next, seg.html);
    }
}
