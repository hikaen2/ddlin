## 1. libeb の取り込み

- [x] 1.1 Cargo workspace (`core`, `app`) を作り、`cargo build` が通ることを確認する
- [x] 1.2 EB Library 4.4.3 のソースを `vendor/eb` に置き、`build.rs` + `cc` + `libz-sys` で Linux 上でビルドし、`cargo build -p core` が通ることを確認する
- [x] 1.3 bindgen で構造体を生成し、必要な関数の FFI 宣言を書き、`eb_initialize_library` → `eb_bind` → `eb_subbook_list` が `~/opt/EPWING/S-I-Dic97` で 10 副本を返すテストを通す
- [x] 1.4 Windows で `cargo build -p core` が通るか spike し、結果 (成功 / 必要な対処) を design.md に追記する

## 2. core: 辞書一覧

- [x] 2.1 `books(root)` を実装し、`~/opt/EPWING` で 20 辞書 (EBX1100: 8, S-I-Dic97: 10, epwing: 1, lvt-out: 1) が返り、`dd266` が無視されるテストを通す
- [x] 2.2 題名を UTF-8 に変換し、「広辞苑　第四版」「三省堂のしゃべる英和辞典」が正しく返るテストを通す
- [x] 2.3 副本ごとの対応検索方式 (prefix / suffix / exact) を返し、テストで確認する

## 3. core: 検索

- [x] 3.1 検索語を UTF-8 から副本の文字コードへ変換する処理を実装し、EUC-JP と EBXA-C のケースをテストする
- [x] 3.2 `search(dict, query, mode)` を実装し (`eb_hit_list` を 0 件まで繰り返す、重複除去)、広辞苑 Prefix「どうぶ」に「動物」が含まれるテストを通す
- [x] 3.3 Exact「どうぶつ」に「動物園」が含まれないこと、EBX1100 英和 Prefix「dog」がヒットすることをテストで確認する
- [x] 3.4 広辞苑 Prefix「あ」の件数と所要時間を計測し、結果を記録する

## 4. core: 本文

- [x] 4.1 libeb の hook で本文を HTML に変換する処理 (テキスト、改行、強調、参照 `<a data-pos>`) を実装し、「どうぶつ【動物】」の HTML をスナップショットテストで確認する
- [x] 4.2 外字を PNG の data URI `<img>` に変換し、ビットマップがない場合は代替記号にする処理を実装し、外字を含む項目でテストする
- [x] 4.3 `read_segment(dict, pos, Forward)` を実装し、約 8000 バイトの後の項目境界で切れること、`next_pos` から続けて読めることをテストする
- [x] 4.4 `read_segment(dict, pos, Backward)` を実装し、前方向のセグメントと連結すると本文が途切れずにつながることをテストする
- [x] 4.5 辞書の先頭・末尾で `prev_pos` / `next_pos` が None になることをテストする
- [x] 4.6 libeb の呼び出しを Mutex で保護する (libeb がグローバル状態を持つため、全辞書で 1 つ)

## 5. app: Tauri と UI

- [x] 5.1 Tauri アプリを作り、`books` / `search` / `read_segment` / 辞書フォルダの取得・設定を command として公開し、`cargo tauri dev` で起動することを確認する
- [x] 5.2 辞書フォルダ設定 (未設定時は英語で促す、選択後に JSON に保存) を実装し、再起動後も保持されることを確認する
- [x] 5.3 辞書ツールバー (題名の先頭 3 文字を表示、1 冊選択、ステータスバーに題名全体) を実装し、クリックで選択が切り替わることを確認する
- [x] 5.4 検索欄、Prefix / Suffix / Exact ボタン (非対応は無効)、見出し一覧、件数、「No results」を実装し、画面で確認する
- [x] 5.5 見出しクリックで本文ペインにその項目から表示し、参照リンクのクリックで参照先から表示し直すことを確認する
- [x] 5.6 本文ペインの下端・上端到達でセグメントを追加し、上方向追加時に `scrollTop` を補正して表示位置が動かないことを確認する

## 6. 統合確認

- [x] 6.1 Linux で specs の全シナリオを手で確認する (広辞苑・EBX1100・新明解)
- [ ] 6.2 Windows と Mac で `cargo tauri build` を行い、起動・検索・連続表示ができることを確認する
