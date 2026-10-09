## Why

手持ちの EPWING / EBXA 辞書を Linux・Windows・Mac で DDwin のように引きたい。既存の GoldenDict-ng は EBXA (EBX1100) を開けず、完全一致寄りの検索のため「どうぶ」で「動物」が出ないなど、DDwin の使い勝手を再現できない。

## What Changes

- 新規デスクトップアプリ ddlin を作る (Linux / Windows / Mac)
- 指定した辞書フォルダ (複数可。辞書そのもの、または辞書をまとめたフォルダ) の EPWING / EBXA 辞書 (副本) を、ツールバーに辞書名を並べる。1 冊を選んで引く
- 前方一致・後方一致の検索。該当する見出しを全件一覧表示する。辞書のメニューも開ける
- 本文ペインは連続表示。上端・下端に達したら前後のセグメントを読み込んでつなげる (無限スクロール)
- 外字・図版 (カラー・白黒) の表示と参照リンクによるジャンプ
- UI は英語。辞書名は辞書データの名前をそのまま表示する
- 対象外: 串刺し検索、辞書グループ、DDPop 相当機能、音声、動画、条件・複合検索、全文検索

## Capabilities

### New Capabilities
- `dictionary-library`: 辞書フォルダの走査、辞書一覧 (ツールバー)、辞書の選択
- `word-search`: 前方・後方一致検索と見出し一覧、メニュー
- `text-view`: 本文の連続表示、セグメント単位の無限スクロール、外字、図版、参照リンク

### Modified Capabilities
(なし)

## Impact

- 新規リポジトリ構成: Rust core + Tauri (HTML/JS UI)
- 依存: EB Library (libeb) の C ソースを同梱しビルド、zlib、Tauri
