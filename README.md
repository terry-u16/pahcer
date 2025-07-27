# Pahcer

AtCoder Heuristic Contest (AHC) のローカルテストを並列実行するツールです。

## 機能

- ローカルテストの初期設定生成
- 並列でローカルテストを実行
- 実行結果の逐次出力
- 実行結果のファイル出力
- ローカルでのベスト解と比較した相対スコア表示
- Optunaとの連携によるパラメータ最適化

## インストール

Rustの実行環境が必要です。[公式サイト](https://www.rust-lang.org/ja)を参考に事前にインストールしてください。

Rustインストール後、以下のコマンドでpahcerをインストールしてください。

```sh
$ cargo install pahcer
```

以下のコマンドが実行できればインストール成功です。

```sh
$ pahcer --version
```

インストールが失敗する場合、以下を順にお試しください。

### Rustのバージョンを更新する

```sh
$ rustup update
```

### `--locked` オプションを付けてインストールする

```sh
$ cargo install pahcer --locked
```

## バージョン指定でのインストール

最新バージョンがうまく動かないなどの理由によりバージョンを指定してインストールするには、以下のコマンドを実行してください。

インストール可能なバージョンの一覧は[crates.io](https://crates.io/crates/pahcer/versions)をご参照ください。

```sh
# 例: cargo install pahcer --version 0.1.1
$ cargo install pahcer --version <VERSION>
```

## バージョンアップ

`cargo-update` を使用する方法と素のCargoを使う方法があります。前者がオススメです。

### `cargo-update` を使う方法

```sh
$ cargo install cargo-update  # 初回のみ必要
$ cargo install-update pahcer
```

### 素のCargoを使う方法

更新がなかった場合でも都度コンパイル処理が走るので少し重いです。

```sh
$ cargo install -f pahcer
```

## アンインストール

アンインストールしたい場合は以下のコマンドを実行してください。

```sh
$ cargo uninstall pahcer
```

## 使い方

### 1. ディレクトリ構成

以下のようにコードとAtCoderの公式テストツールを配置してください。

この配置に従わなくても構いませんが、設定ファイルの編集が必要になります。

ただしここで、"AtCoder提供の公式ローカルテストツール"とは、Windows用のコンパイル済みバイナリではなく、Rust言語で書かれたソースファイルの方を指すものとします。
#### C++

```text
ahc000       (プロジェクトルート)
├ main.cpp   (解答プログラムのコード)
└ tools      (AtCoder提供の公式ローカルテストツール)
  ├ src
  └ in
```

#### Python

```text
ahc000       (プロジェクトルート)
├ main.py    (解答プログラムのコード)
└ tools      (AtCoder提供の公式ローカルテストツール)
  ├ src
  └ in
```

#### Rust

```text
ahc000       (プロジェクトルート)
├ src
│ └ main.rs  (解答プログラムのコード)
├ targets    (ビルド成果物フォルダ)
├ Cargo.toml
└ tools      (AtCoder提供の公式ローカルテストツール)
  ├ src
  └ in
```

#### Go

```text
ahc000       (プロジェクトルート)
├ main.go   (解答プログラムのコード)
└ tools      (AtCoder提供の公式ローカルテストツール)
  ├ src
  └ in
```

### 2. 初期設定

以下を実行して、初期設定を行います。

```sh
$ pahcer init -p <PROBLEM_NAME> -o <OBJECTIVE> -l <LANGUAGE> [-i]
```

- `<PROBLEM_NAME>` にはコンテスト名を入れてください。
- `<OBJECTIVE>` にはスコアが大きい方が良いか小さい方が良いかを指定してください。
  - `max` : スコアが大きい方が良い
  - `min` : スコアが小さい方が良い
- `<LANGUAGE>` にはお使いの言語を入力してください。現在以下のオプションが指定可能です。
  - `cpp` : C++
  - `python` : Python
  - `rust` : Rust
    - Rustを使用する場合、 `cargo.toml` の `package.name` と `patcher init` の `<PROBLEM_NAME>` を一致させないと設定ファイルの編集が必要になります。
    - また、Rustで `targets` ディレクトリがプロジェクトルート直下にない場合も同様です。cargo-competeを使用している場合などは注意してください。
  - `go` : Go
- `-i` オプションはインタラクティブ問題の時に設定してください。

実行すると、設定ファイルが `./pahcer_config.toml` に生成されます。また、テストケースの実行結果が格納される `./pahcer` ディレクトリが生成されます。

#### 例

AHC039 （非インタラクティブ問題、スコア最大化）でC++を使用

```sh
$ pahcer init -p ahc039 -o max -l cpp
```

AHC030 （インタラクティブ問題、スコア最小化）でPythonを使用

```sh
$ pahcer init -p ahc030 -o min -l python -i
```

### 3. テストケース実行

以下のコマンドを実行するとテストケースが並列で実行されます。

```sh
$ pahcer run
```

実行中、各ケースの実行結果がコンソールに表形式で逐次出力されます。各列の内容は以下の通りです。

- `Progress` : テストケース実行の進行状況です。
- `Seed` : 実行したテストケースのseed値です。
- `Case Score` : 当該テストケースのスコアです。
  - `Score` : 実スコア（正の整数値のみ許容）です。0点の場合はWA扱いとなります。
  - `Relative` : ローカルでのベストスコアを100としたときの相対スコアです。
    - `OBJECTIVE = max` のときは `100 * YOURS / BEST` 、 `OBJECTIVE = min` のときは `100 * BEST / YOURS` で計算されます。
- `Average Score` : その時点までの平均スコアです。
  - `Score` : 実スコアの平均値です。
  - `Relative` : 相対スコアの平均値です。
- `Exec. Time` : 実行時間（ミリ秒表示）です。並列実行数などにより変化しうるので参考程度にご覧ください。

このとき、並列実行を行っている都合上seedの順番が実行ごとに変化することに注意してください。途中で実行を中断する場合は `Ctrl+C` を押してください。

実行後、以下の情報が表示されます。

- `Average Score` : 実スコアの平均値です。
- `Average Score (log10)` : 実スコアの対数を取った値の平均値です。相対スコア問題の評価などに活用いただけます。
- `Average Relative Score` : 相対スコアの平均値です。
- `Accepted` : Acceptされたケース数です。正の点数を取ったテストケースがAcceptedと見なされます。実行時間が長くてもTLE扱いにはなりませんのでご注意ください。
- `Max Execution Time` : 実行時間の最大値です。

また、実行後以下の3ファイルが生成または追記されます。

- `./pahcer/summary.md` : 実行結果のサマリが表形式で記録されたファイルです。
- `./pahcer/best_scores.json` : ローカルでのベストスコアが保存されたJSONファイルです。
- `./pahcer/json/result_*.json` : 実行結果の詳細が記録されたJSONファイルです。

デフォルトでは、 seed=0 から seed=99 までの100ケースが実行されます。カスタマイズしたい場合やうまく動かない場合は `./pahcer_config.toml` を編集してください。

### 4. 実行結果確認

以下のコマンドを実行すると、最新10件の実行結果が表形式で確認できます。

```sh
# 最新10件の結果を表示
$ pahcer list
```

オプションを指定することで、表示件数を変更することもできます。

```sh
# 最新5件の結果を表示
$ pahcer list -n 5

# 全ての結果を表示
$ pahcer list -a
```

### 5. Optunaとの連携によるパラメータ最適化（オプション）

Optunaとの連携によるパラメータ最適化も可能です。詳細は `./optuna-sample/README.md` をご参照ください。

## コマンド

pahcerの実行コマンド一覧です。

### `pahcer init`

pahcerの初期設定を行います。

```sh
$ pahcer init [OPTIONS] -p <PROBLEM_NAME> -o <OBJECTIVE> -l <LANGAGE>
```

#### オプション

- `-p`, `--problem`
  - コンテスト名を指定します（必須）。
- `-o`, `--objective`
  - スコアが大きい方が良いか小さい方が良いかを指定します（必須）。
  - 以下のいずれかが指定可能です。
    - `max` : スコアが大きい方が良い
    - `min` : スコアが小さい方が良い
- `-l`, `--language`
  - 解答プログラムの言語を入力します（必須）。
  - 現在以下のオプションが使用可能です。
    - `cpp` : C++
    - `python` : Python
    - `rust` : Rust
    - `go` : Go
- `-i`, `--interactive`
  - インタラクティブ問題の際に指定します。

以下でヘルプが出せます。

```sh
$ pahcer init -h
```

#### 実行例

```sh
$ pahcer init -p ahc030 -o min -l python -i
```

### `pahcer run`

テストケースを並列実行します。

```sh
$ pahcer run [OPTIONS]
```

#### オプション

- `-c`, `--comment`
  - テストケースにコメントを付与します。
  - コメントはサマリファイルなどにスコアとともに書き出されるため、解答コードの内容のメモなどにご活用ください。
- `-t`, `--tag`
  - テスト実行時に自動でGitタグを作成します。Gitがインストールされている必要があります。
  - タグ名を指定しない場合、`pahcer/{8文字のランダム文字列}`形式で自動生成されます（例: `pahcer/aB3xK9mZ`）。
  - タグ名を指定した場合、`pahcer/<tag-name>` という形式で作成されます（例: `pahcer run -t my-solution` → `pahcer/my-solution`）。
  - 作成したタグは `pahcer prune` で一括削除可能です。
- `-j`, `--json`
  - 各ケースの実行結果を表形式ではなくJSON形式でコンソールに出力します。
  - Optunaをはじめとした外部アプリケーションとの連携にご活用ください。
- `--shuffle`
  - テストケースの実行順序をシャッフルします。
  - Optunaの[WilcoxonPruner](https://tech.preferred.jp/ja/blog/wilcoxonpruner/)との連携などに使います。
- `--setting-file`
  - 読み込む設定ファイル（ `./pahcer_config.toml` ）のパスをデフォルトから変更します。
  - 一時的に実行設定を変更する場合などに使います。
- `--freeze-best-scores`
  - ベストスコアの更新を行わないようにします。
- `--no-result-file`
  - 全ケース完了後に実行結果のファイル出力を行わないようにします。
- `--no-compile`
  - 起動時にコンパイル処理を行わないようにします。

以下でヘルプが出せます。

```sh
$ pahcer init -h
```

#### 実行例

```sh
$ pahcer run -c 焼きなまし高速化バージョン -j --shuffle --setting-file settings.toml --freeze-best-scores --no-result-file
```

### `pahcer list`

過去のテスト実行結果を表形式で一覧表示します。

```sh
$ pahcer list [OPTIONS]
```

実行結果のJSONファイルを読み込み、以下の情報を含むテーブルを表示します。

- `Time` : テスト実行日時
- `AC/All` : Accept数/全テストケース数
- `Avg Score` : 平均スコア
- `Avg Rel.` : 平均相対スコア（最新のベストスコアを元に再計算されます）
- `Max Time` : 最大実行時間
- `Tag` : Gitタグ名（`pahcer/`プレフィックスは除去して表示）
- `Comment` : テスト実行時のコメント

#### オプション

- `-n`, `--number`
  - 表示する結果の件数を指定します（デフォルト: 10）。
- `-a`, `--all`
  - 全ての結果を表示します。
- `--setting-file`
  - 読み込む設定ファイル（ `./pahcer_config.toml` ）のパスをデフォルトから変更します。

以下でヘルプが出せます。

```sh
$ pahcer list -h
```

#### 実行例

```sh
# 最新10件の結果を表示
$ pahcer list

# 最新5件の結果を表示
$ pahcer list -n 5

# 全ての結果を表示
$ pahcer list -a
```

### `pahcer prune`

pahcerが作成したGitタグを全て削除します。

```sh
$ pahcer prune
```

このコマンドは `pahcer/*` パターンにマッチするタグを全て削除します。手動で作成したタグには影響しません。

#### 実行例

```sh
$ pahcer prune
Deleted tag: pahcer/aB3xK9mZ
Deleted tag: pahcer/7pQw2nVj
Deleted tag: pahcer/my-solution
```

## 設定ファイル

設定ファイル `./pahcer_config.toml` の内容を説明します。

### `general`

全般に関する設定です。

#### `version`

設定ファイルのバージョンです。

### `problem`

問題固有の項目に関する設定です。

#### `problem_name`

問題の名前（コンテスト名）です。

#### `objective`

スコアが大きい方が良いか小さい方が良いかを指定します。以下のいずれかが指定可能です。
- `Max` : スコアが大きい方が良い
- `Min` : スコアが小さい方が良い

#### `score_regex`

スコアの抽出を行う正規表現です。

pahcerは各実行ステップにおける標準出力・標準エラー出力の内容を全て読み込み、 `score_regex` に一致した行からスコアを抽出します。そのような行が複数存在する場合は最も最後の行が優先されます（同実行ステップで標準出力・標準エラー出力両方に存在する場合は標準エラー出力が優先）。なお、一致する行が1つも存在しなかった場合は `WA` となります。

### `test`

テストケースの実行に関する設定です。

#### `start_seed`

テストケースの開始seed値を指定します。

#### `end_seed`

テストケースの終了seed値を指定します。 `start_seed` より大きい値でなければなりません。

[start_seed, end_seed) の半開区間が実行されるため、 **`end_seed` は区間に含まれない** ことに注意してください。

#### `threads`

並列実行数を指定します。 `0` を指定すると実行しているマシンの物理CPU数と同じ値となります。

#### `out_dir`

全ケース終了後の結果ファイルの出力先ディレクトリを指定します。

#### `compile_steps`

`pahcer run` を実行したときに一度だけ行われるコンパイル実行のステップです。複数設定することが可能で、その場合は上から順に逐次実行されます。

##### `program`

コンパイルステップで実行されるプログラム名です。

##### `args`

コンパイルステップでプログラムに渡されるコマンドライン引数です。配列の形で渡します。

##### `current_dir`

コンパイルステップの実行ディレクトリです。省略が可能で、省略した場合はカレントディレクトリとなります。

#### `test_steps`

テストケース実行時に行われるステップです。複数設定することが可能で、その場合は上から順に逐次実行されます。

なお、 `args`, `stdin`, `stdout`, `stderr` にはプレースホルダーが設定可能で、以下のように展開されます。

- `{SEED}` : シード値（例: `{SEED}.txt` -> `1.txt`）
- `{SEED04}` : 0で4桁にパディングされたシード値（例: `{SEED04}.txt` -> `0001.txt`）

##### `program`

テストステップで実行されるプログラム名です。

##### `args`

テストステップでプログラムに渡されるコマンドライン引数です。配列の形で渡します。

##### `stdin`

テストステップでプログラムに渡される標準入力の内容が記録されたファイルを指定します。省略が可能で、省略した場合は標準入力に何も渡しません。

##### `stdout`

テストステップでプログラムから出力される標準出力の記録先ファイルを指定します。省略が可能で、省略した場合はファイル出力を行いません（スコア抽出にのみ使用されます）。

##### `stderr`

テストステップでプログラムから出力される標準エラー出力の記録先ファイルを指定します。省略が可能で、省略した場合はファイル出力を行いません（スコア抽出にのみ使用されます）。

##### `measure_time`

実行時間の計測対象か否かをbool値で指定します。 `true` が指定されたテストステップの実行時間の合計値が最終的に出力されます。

## ライセンス

[MIT](https://opensource.org/license/MIT)または[Apache-2.0](https://www.apache.org/licenses/LICENSE-2.0)のデュアルライセンスです。

## その他

- AHC期間中は質問・要望・不具合対応ができない可能性が高いです。ご了承ください。
- pahcerという名前は pacer (伴走者) + ahc から来ています。ペーサーと呼んでください。
- 作者は `pahcer` をよく `pacher` とtypoします。必要に応じてエイリアスを設定するとよいです。
