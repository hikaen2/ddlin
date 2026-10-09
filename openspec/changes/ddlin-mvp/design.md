## Context

新規リポジトリで、既存コードはない。動機は proposal.md の Why を参照。手元の辞書は EPWING (S-I-Dic97, LogoVista 変換の新明解) と EBXA (EBX1100、JIS X 0208 + GB 2312) で、いずれも Linux の `ebinfo` (EB Library 4.4.3) で読めることを確認済み。対象 OS は Linux / Windows / Mac。

## Goals / Non-Goals

**Goals:**
- 3 OS で同じコードから動くこと
- EPWING の読み取りは EB Library (libeb) に任せ、自前のコードを薄く保つこと

**Non-Goals:**
- EPWING 形式の再実装
- 見出し一覧の仮想スクロール (必要になるまで作らない)
- 本文のプリフェッチ

## Decisions

### 構成: Rust core + Tauri

```
+--------------------------------------+
| ui/  HTML/CSS/JS (Tauri WebView)     |
|   ツールバー / 検索欄+一覧 / 本文     |
+--------------------------------------+
        | Tauri command (JSON)
+--------------------------------------+
| core (Rust crate, Tauri 非依存)      |
|   books / search / read_segment      |
|   本文 -> HTML 変換                  |
+--------------------------------------+
        | FFI
+--------------------------------------+
| libeb (C ソース同梱, build.rs + cc)  |
+--------------------------------------+
```

- 本文はリッチテキスト (外字・参照・強調) なので HTML で描くのが最も素直。
- core は Tauri に依存させず、テストやデバッグ用 CLI から直接呼べるようにする。
- 代替案: Python + GTK4 (Windows / Mac の配布が困難), Python + PySide6 (libeb を OS ごとに配布する手間), C++ + Qt (利用者の意向で不採用), Rust で EPWING 再実装 (作業量が大きすぎる)。

### libeb の取り込み

EB Library 4.4.3 の C ソースを `vendor/eb` に同梱し、`build.rs` で `cc` crate によりコンパイルする。`config.h` は OS ごとに手で用意する。zlib は `libz-sys` を使う。FFI 宣言は必要な関数だけ手書きし、`EB_Book` / `EB_Appendix` などの構造体は bindgen で生成する (構造体のサイズを手で合わせない)。

- 代替案: システムの libeb にリンク。Linux では楽だが Windows / Mac では入手できないため不採用。

### core API

```
books(root)                    -> [Dict { id, title, methods }]
search(dict, query, mode)      -> [Hit { heading_html, pos }]
read_segment(dict, pos, dir)   -> { html, prev_pos?, next_pos? }
```

- `pos` は libeb の `EB_Position` (page, offset)。
- `search` は `eb_search_word` / `eb_search_endword` / `eb_search_exactword` の後、`eb_hit_list` を 0 件になるまで繰り返して全件集める。同じ本文位置の重複は除く。
- 検索語は UTF-8 で受け取り、副本の文字コード (EUC-JP、EBXA-C なら GB 2312 も) に変換して渡す。
- 題名・見出し・本文は UTF-8 に変換して返す。

### セグメント

- 下方向: `eb_seek_text(pos)` の後、`eb_read_text` で項目の区切りを越えて読み、約 8000 バイト (DDwin の `BufferSize=8000` に合わせる) に達した後の最初の項目境界で切る。`next_pos` は `eb_tell_text` で得る。
- 上方向: `eb_backward_text` で項目単位にさかのぼり、同じ量だけ読んで先頭側のセグメントとする。
- 本文 -> HTML 変換は libeb の hook (`EB_HOOK_*`) で行う: 外字 → `<img>` (data URI、PNG に変換)、参照 → `<a data-pos>`、強調 → `<b>` など。

### UI のスクロール

- `scroll` イベントで `scrollTop == 0` または下端到達を検出し、読み込み中フラグで多重呼び出しを防ぐ。
- 上方向に追加するときは、追加前後の `scrollHeight` の差を `scrollTop` に足して表示位置を保つ。`overflow-anchor` は WebKit (Linux / Mac) で効かないため頼らない。

### 設定

辞書フォルダのパスだけを、OS の設定ディレクトリの JSON ファイルに保存する (Tauri の app config dir)。

## Risks / Trade-offs

- [libeb が Windows / Mac でビルドできない] → 最初に Windows (MSVC) でビルドの spike を行う。だめなら MinGW、最終手段として該当部分を Rust で再実装。
- [EBXA-C (GB 2312) の検索語変換が正しくない] → EBX1100 で「dog」などを検索する受け入れテストを早い段階で通す。
- [`eb_backward_text` で項目境界が正しく得られない辞書がある] → 上方向は境界が見つからなければ読み込まない (エラーにしない)。
- [見出し一覧が数万件になり UI が重い] → まず素の DOM で計測し、問題があれば仮想リストにする。
- [libeb はスレッドセーフでない。本文キャッシュなどをプロセス全体で共有している] → すべての辞書をまとめて 1 つの `Mutex` で保護し、libeb を同時に呼ばない。辞書ごとの `Mutex` では、別の辞書を同時に読んだときに `EB_ERR_UNEXP_TEXT` になった。

## Spike results

- Windows (task 1.4): Linux 上で `x86_64-pc-windows-gnu` (MinGW) にクロスコンパイルし、wine で実行して S-I-Dic97 の 10 副本を取得できた。必要な対処は Windows ターゲットで `DOS_FILE_PATH` / `WIN32` を定義し、`getdcwd` を `_getdcwd` に読み替えることの 2 点。設定ヘッダは Linux で生成した `config.h` のままで通った。MSVC ターゲットは未確認なので、Windows 版は当面 MinGW (`-gnu`) でビルドする。libeb は ANSI のファイル API を使うため、日本語を含むパスは Windows で開けない可能性がある (未確認)。

- セグメントの大きさはタグと外字画像を除いた本文の長さで数える。HTML の長さで数えると、外字の多い辞書 (EBX1100) では 3 項目ほどで 8000 バイトに達してしまった。
- 見出し一覧の件数 (task 3.4): 広辞苑 Prefix「あ」は 7,643 件、リリースビルドで約 70 ms (デバッグビルドで約 0.3 s)。全件取得で問題ない。
- `eb_backward_text` は辞書の最初の項目で `EB_ERR_FAIL_READ_TEXT` を返すことがある。上方向の読み込みではエラーを先頭到達として扱う。

- 統合確認 (task 6.1): Linux で `cargo tauri build` から deb / rpm を作成できた (AppImage は外部ツールのダウンロードが必要)。UI での日本語入力 (IME) による検索は自動操作できなかったため、日本語の検索語は core のテストで確認した。
- Windows 版アプリ (task 6.2): `x86_64-pc-windows-gnu` でクロスビルドした `ddlin.exe` は、wine に WebView2 ランタイムが無いため起動を確認できなかった。Windows 実機での確認が必要。Mac は未確認。

## Open Questions

- 外字の表示サイズ (16 / 24 ドットのどちらを使うか) は実際の見た目で決める。
