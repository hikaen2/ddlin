## Purpose

辞書フォルダから EPWING / EBXA 辞書を見つけ、ツールバーに並べて 1 冊を選べるようにする。

## ADDED Requirements

### Requirement: Dictionary folder
システムは利用者が指定した辞書フォルダを保存し、次回起動時にも使用しなければならない (SHALL)。

#### Scenario: Folder is remembered
- **WHEN** 利用者が辞書フォルダ `~/opt/EPWING` を指定してアプリを再起動する
- **THEN** 再起動後も同じフォルダの辞書が一覧に表示される

#### Scenario: No folder configured
- **WHEN** 辞書フォルダが未設定の状態で起動する
- **THEN** フォルダを選ぶよう英語で促す表示が出る

### Requirement: Dictionary discovery
システムは辞書フォルダ直下の各ディレクトリを EPWING / EBXA 書籍として開き、そのすべての副本を辞書として登録しなければならない (SHALL)。開けないディレクトリは無視しなければならない (SHALL)。

#### Scenario: Multiple books and subbooks
- **WHEN** 辞書フォルダに `EBX1100` (8 副本), `S-I-Dic97` (10 副本), `epwing` (1 副本) がある
- **THEN** 19 個の辞書が登録される

#### Scenario: Non-dictionary directory
- **WHEN** 辞書フォルダに辞書でないディレクトリがある
- **THEN** そのディレクトリは無視され、他の辞書は通常どおり登録される

### Requirement: Dictionary toolbar
システムは登録した辞書をツールバーに並べ、1 冊だけを選択状態にしなければならない (SHALL)。辞書名は辞書データにある題名を翻訳せずにそのまま表示しなければならない (SHALL)。

#### Scenario: Japanese title shown as is
- **WHEN** 広辞苑 第四版の副本が登録されている
- **THEN** ツールバーに「広辞苑　第四版」と表示される

#### Scenario: Selecting a dictionary
- **WHEN** 利用者がツールバーで辞書をクリックする
- **THEN** その辞書だけが選択状態になり、以後の検索はその辞書に対して行われる

### Requirement: English UI
システムの UI 文言 (メニュー、ボタン、メッセージ) は英語でなければならない (SHALL)。辞書名と辞書の内容はこの対象外とする。

#### Scenario: UI labels
- **WHEN** アプリを起動する
- **THEN** 検索モードのボタンなどの UI 文言が英語で表示される
