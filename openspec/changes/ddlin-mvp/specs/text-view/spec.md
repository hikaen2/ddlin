## Purpose

辞書の本文を DDwin の連続表示のように、項目の区切りを越えて前後へ読み進められるように表示する。

## ADDED Requirements

### Requirement: Continuous display
システムは本文ペインで、選んだ項目から辞書本文の並び順に後続の項目を続けて表示しなければならない (SHALL)。

#### Scenario: Following entries shown
- **WHEN** 広辞苑 第四版で「どう‐ぶ【童舞】」を開く
- **THEN** 童舞の本文の後に、辞書本文で次に並ぶ項目が続けて表示される

### Requirement: Segment loading at edges
本文ペインが下端までスクロールされたとき、システムは次のセグメントを読み込んで末尾に追加しなければならない (SHALL)。上端までスクロールされたとき、前のセグメントを読み込んで先頭に追加しなければならない (SHALL)。既に表示している内容は残さなければならない (SHALL)。

#### Scenario: Reaching bottom
- **WHEN** 本文ペインを下端までスクロールする
- **THEN** 次のセグメントが末尾に追加され、さらに下へ読み進められる

#### Scenario: Reaching top
- **WHEN** 本文ペインを上端までスクロールする
- **THEN** 前のセグメントが先頭に追加され、それまで見ていた行は画面上の同じ位置に留まる

#### Scenario: Start or end of dictionary
- **WHEN** 辞書本文の先頭または末尾に達している
- **THEN** それ以上は読み込まれず、エラーも表示されない

### Requirement: Gaiji
システムは本文と見出しの外字を辞書の外字ビットマップで表示しなければならない (SHALL)。ビットマップを得られない外字は代替記号で表示しなければならない (SHALL)。

#### Scenario: Gaiji bitmap
- **WHEN** 外字を含む項目を表示する
- **THEN** 外字が周囲の文字と同じ行に画像として表示される

### Requirement: Graphics
システムは本文中のカラー図版 (BMP・JPEG) と白黒図版を、本文の該当位置に画像として表示しなければならない (SHALL)。読み出せない図版は表示せずに読み飛ばさなければならない (SHALL)。

#### Scenario: Colour photo
- **WHEN** 広辞苑 第四版のメニューから「カラー写真・図版」→「人物」→「青野季吉」を開く
- **THEN** 本文ペインに青野季吉の写真が表示される

#### Scenario: Monochrome drawing
- **WHEN** 広辞苑 第四版のメニューから「分野別図版」→「鳥」→「アオジ」を開く
- **THEN** 本文ペインにアオジの白黒の絵が表示される

### Requirement: Reference links
システムは本文中の参照をリンクとして表示し、クリックされたら参照先の項目から連続表示し直さなければならない (SHALL)。

#### Scenario: Follow reference
- **WHEN** 本文中の「→都道府県」のような参照リンクをクリックする
- **THEN** 本文ペインが参照先の項目から表示し直される
