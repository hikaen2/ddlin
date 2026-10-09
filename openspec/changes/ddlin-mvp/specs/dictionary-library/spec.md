## Purpose

辞書フォルダから EPWING / EBXA 辞書を見つけ、ツールバーに並べて 1 冊を選べるようにする。

## ADDED Requirements

### Requirement: Dictionary folder
システムは利用者が指定した複数の辞書フォルダを保存し、次回起動時にも使用しなければならない (SHALL)。利用者はフォルダを追加・削除できなければならない (SHALL)。

#### Scenario: Folders are remembered
- **WHEN** 利用者が辞書フォルダ `~/opt/EPWING/EBX1100/` と `~/opt/EPWING/S-I-Dic97/` を指定してアプリを再起動する
- **THEN** 再起動後も両方のフォルダの辞書が一覧に表示される

#### Scenario: Removing a folder
- **WHEN** 利用者が辞書フォルダの一覧から `~/opt/EPWING/S-I-Dic97/` を削除する
- **THEN** そのフォルダの辞書はツールバーから消え、残りのフォルダの辞書だけが表示される

#### Scenario: No folder configured
- **WHEN** 辞書フォルダが未設定の状態で起動する
- **THEN** フォルダを追加するよう英語で促す表示が出る

### Requirement: Dictionary discovery
システムは各辞書フォルダを、指定された順に EPWING / EBXA 書籍として開き、そのすべての副本を辞書として登録しなければならない (SHALL)。書籍として開けないフォルダは、その直下の各ディレクトリを書籍として開かなければならない (SHALL)。開けないディレクトリは無視し、同じ書籍は 1 度だけ登録しなければならない (SHALL)。

#### Scenario: Book folders
- **WHEN** 辞書フォルダとして `EBX1100` (8 副本) と `S-I-Dic97` (10 副本) を指定する
- **THEN** 18 個の辞書がこの順に登録される

#### Scenario: Folder of books
- **WHEN** 辞書フォルダの直下に `EBX1100` (8 副本), `S-I-Dic97` (10 副本), `epwing` (1 副本) がある
- **THEN** 19 個の辞書が登録される

#### Scenario: Non-dictionary directory
- **WHEN** 辞書フォルダに辞書でないディレクトリがある
- **THEN** そのディレクトリは無視され、他の辞書は通常どおり登録される

### Requirement: Dictionary toolbar
システムは登録した辞書をツールバーに並べ、1 冊だけを選択状態にしなければならない (SHALL)。ツールバーには辞書データにある題名の先頭 3 文字を翻訳せずに表示し、選択中の辞書の題名全体を画面下のステータスバーに表示しなければならない (SHALL)。

#### Scenario: Short title on toolbar
- **WHEN** 広辞苑 第四版の副本が登録されている
- **THEN** ツールバーに「広辞苑」と表示される

#### Scenario: Full title in status bar
- **WHEN** 利用者がツールバーで「広辞苑」を選ぶ
- **THEN** ステータスバーに「広辞苑　第四版」と表示される

#### Scenario: Selecting a dictionary
- **WHEN** 利用者がツールバーで辞書をクリックする
- **THEN** その辞書だけが選択状態になり、以後の検索はその辞書に対して行われる

### Requirement: English UI
システムの UI 文言 (メニュー、ボタン、メッセージ) は英語でなければならない (SHALL)。辞書名と辞書の内容はこの対象外とする。

#### Scenario: UI labels
- **WHEN** アプリを起動する
- **THEN** 検索モードのボタンなどの UI 文言が英語で表示される
