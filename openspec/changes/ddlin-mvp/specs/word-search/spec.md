## Purpose

選択中の辞書を前方一致・後方一致で検索して該当する見出しを全件一覧にし、辞書のメニューを開けるようにする。

## ADDED Requirements

### Requirement: Search modes
システムは選択中の辞書に対して Prefix (前方一致)、Suffix (後方一致) の検索を提供しなければならない (SHALL)。辞書がその検索方式に対応していない場合、その方式を選べるようにしてはならない (SHALL NOT)。

#### Scenario: Prefix search
- **WHEN** 広辞苑 第四版を選び、Prefix で「どうぶ」を検索する
- **THEN** 一覧に「どう‐ぶ【童舞】」と「どうぶつ【動物】」が含まれる

#### Scenario: Suffix search
- **WHEN** 広辞苑 第四版を選び、Suffix で「どうぶつ」を検索する
- **THEN** 一覧に「どうぶつ【動物】」が含まれる

#### Scenario: EBXA dictionary
- **WHEN** 三省堂のしゃべる英和辞典 (EBX1100) を選び、Prefix で「dog」を検索する
- **THEN** 「dog」で始まる見出しが一覧に表示される

### Requirement: Menu
システムは選択中の辞書にメニューがあれば、それを本文ペインに表示する手段を提供しなければならない (SHALL)。メニューの各項目はリンクとして表示し、クリックで項目の内容を表示しなければならない (SHALL)。単語検索が無くメニューがある辞書では、辞書を選んだときにメニューを表示しなければならない (SHALL)。

#### Scenario: Open the menu
- **WHEN** 広辞苑 第四版を選び、Menu を押す
- **THEN** 本文ペインに「●地理」などのメニューが表示される

#### Scenario: Dictionary with only a menu
- **WHEN** 付属資料を選ぶ
- **THEN** 本文ペインに「１．凡例」などのメニューが表示され、クリックするとその内容が表示される

### Requirement: Complete result list
システムは検索条件に該当する見出しを件数制限なしですべて一覧に表示しなければならない (SHALL)。同じ本文位置を指す重複した見出しは 1 件にまとめなければならない (SHALL)。

#### Scenario: Short prefix
- **WHEN** 広辞苑 第四版で Prefix「あ」を検索する
- **THEN** 「あ」から「あんわ」までの該当見出しがすべて一覧に表示される

#### Scenario: Result count
- **WHEN** 検索が終わる
- **THEN** 該当件数が表示される

### Requirement: Empty results
該当する見出しがない場合、システムは英語でその旨を表示しなければならない (SHALL)。

#### Scenario: No match
- **WHEN** どの見出しにも該当しない語を検索する
- **THEN** 一覧は空になり「No results」と表示される

### Requirement: Opening an entry
利用者が一覧の見出しを選んだとき、システムはその見出しの本文を本文ペインに表示しなければならない (SHALL)。

#### Scenario: Select heading
- **WHEN** 一覧で「どうぶつ【動物】」をクリックする
- **THEN** 本文ペインの先頭に動物の項目が表示される
