## Purpose

選択中の辞書を前方一致・後方一致・全文で検索して該当する見出しを一覧にし、辞書のメニューを開けるようにする。

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

### Requirement: Full-text search
システムは選択中の辞書の本文を先頭から読み、検索語を含む項目を本文の順に一覧にする Full 検索を提供しなければならない (SHALL)。照合はタグを除いた本文に対して行い、英字の大文字・小文字を区別してはならない (SHALL NOT)。一覧の見出しは各項目の 1 行目とする。検索中は進み具合を表示し、利用者が止められなければならない (SHALL)。結果が 1000 件に達したら止めなければならない (SHALL)。外字は照合の対象外とし、本文がまとまりに分かれた辞書では最初のまとまりだけを対象とする。

#### Scenario: Phrase in the body
- **WHEN** 広辞苑 第四版を選び、Full で「空腹をみたすには足りない」を検索する
- **THEN** 一覧に「あさ‐はら【朝腹】」が表示される

#### Scenario: Stopping a search
- **WHEN** Full 検索の途中で Stop を押す
- **THEN** 検索が止まり、それまでに見つかった項目が一覧に残る

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
