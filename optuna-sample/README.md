# Optunaによるハイパーパラメータ最適化サンプル

[Optuna](https://www.preferred.jp/ja/projects/optuna/)と連携してパラメータの最適化を行うサンプルコードです。

Python側からOptunaを呼び出し、その中でpahcerを使用しています。

## クイックスタート

f(x, y) = x^2 + y^2 という関数に対し、`f(x, y)` が最小となる整数変数 `x` と浮動小数点変数 `y` をOptunaで探索するサンプルを実行します。（スコアは整数である必要があるため、これを10^6倍したものをスコアとしています。）

Optunaにより5分間パラメータチューニングが行われ、その結果となるパラメータ (x, y) の組が出力されます。

### uvを使う方法（おすすめ）

uvはPythonの高速なパッケージマネージャです。

[uvの公式サイト](https://docs.astral.sh/uv/#getting-started)を参考にインストールした後、以下のコマンドを実行してください。

```sh
$ uv run optimize.py
```

### その他の方法

pipなどで `optuna` および `scipy` をインストールしたのち、 `optimize.py` を実行してください。環境によっては `.python-version` を削除または編集する必要があるかもしれません。

```sh
$ pip install optuna scipy
$ python optimize.py
```

## 使い方

コンテストで使用するためには、 `optimize.py` 内の **`TODO:` と書かれた4つの関数を編集する** 必要があります。

あわせて、解答プログラムでパラメータを受け取る処理の追加および `pahcer_config.toml` の編集も必要です。

### `optimize.py` の編集

#### パラメータ生成

`optimize.py` のパラメータ生成部分をチューニングしたいパラメータに合わせて編集します。

サンプルのように、整数を生成する場合は `trial.suggest_int(変数名, 最小値, 最大値)` 、浮動小数点数を生成する場合は `trial.suggest_float(変数名, 最小値, 最大値)` と記述してください。より詳しい使い方は[公式リファレンス](https://optuna.readthedocs.io/en/stable/reference/generated/optuna.trial.Trial.html)等をご参照ください。

```python
# TODO: Write parameter suggestions here
def generate_params(trial: optuna.trial.Trial) -> dict[str, str]:
    # for more information, see https://optuna.readthedocs.io/en/stable/reference/generated/optuna.trial.Trial.html
    params = {
        "AHC_X": str(trial.suggest_int("x", -10, 10)),
        "AHC_Y": str(trial.suggest_float("y", -10.0, 10.0)),
    }

    return params
```

#### スコアの定義

Optunaの目的関数となるスコアを定義します。pahcerから各ケースの実行結果がJSON形式で渡されてくるので、そこから必要な情報を抽出します。

基本は実スコア（`absolute_score`）で良いですが、相対スコア形式となる長期コンテストでは実スコアの大きいケースが過大に評価される可能性があるため、必要に応じて実スコアの対数を取ったものや相対スコアを使用すると良いでしょう。

```python
# TODO: Customize the score extraction code here
def extract_score(result: dict[str, str]) -> float:
    absolute_score = result["score"]  # noqa: F841
    log10_score = math.log10(absolute_score) if absolute_score > 0.0 else 0.0  # noqa: F841
    relative_score = result["relative_score"]  # noqa: F841

    score = absolute_score  # for absolute score problems
    # score = log10_score       # for relative score problems (alternative)
    # score = relative_score    # for relative score problems

    return score
```

#### スコア最大化・最小化の設定

スコアを最大化するか最小化するかを設定します。

最小化する場合は `minimize` 、最大化する場合は `maximize` を指定してください。

```python
# TODO: Set the direction to minimize or maximize
def get_direction() -> str:
    direction = "minimize"
    # direction = "maximize"
    return direction
```

#### パラメータ試行の実行時間・試行回数の設定

チューニングにかける実行時間または試行回数を設定します。

実行時間を指定する場合は `timeout` を秒数で、試行回数を指定する場合は `n_trials` を指定してください。

```python
# TODO: Set the timeout (seconds) or the number of trials
def run_optimization(study: optuna.study.Study) -> None:
    study.optimize(Objective(), timeout=300)
    # study.optimize(Objective(), n_trials=100)
```

### パラメータの受け取り処理

パラメータが環境変数経由で解答プログラムに送られてくるので、それを受け取る処理を書く必要があります。

Pythonの場合は `solution.py` のように `os.getenv()` を使うとよいでしょう。環境変数が渡されなかった場合にエラーとならないよう注意してください。

```python
import os

# Default values for x and y if the environment variables are not set
DEFAULT_X = 1
DEFAULT_Y = 1.0

# Get the parameters from the environment variables
x = int(os.getenv("AHC_X") or DEFAULT_X)
y = float(os.getenv("AHC_Y") or DEFAULT_Y)

# f(x, y) = x^2 + y^2
f = x * x + y * y
```

### `pahcer_config.toml` の編集

`pahcer_config.toml` の編集については、pahcerの `README.md` をご参照ください。

## その他

### WilcoxonPrunerについて

`optimize.py` では、AHCと相性の良い枝刈りアルゴリズムである `WilcoxonPruner` を使用しています。 `WilcoxonPruner` に関する詳細は[PFNの公式ブログ](https://tech.preferred.jp/ja/blog/wilcoxonpruner/)をご参照ください。

### pahcerに渡している引数について

`optimize.py` では、内部で以下のような形でpahcerを呼び出しています。このときに渡している引数の意図について説明します。

```sh
$ pahcer run --json --shuffle --no-result-file --freeze-best-scores
```

#### `--json`

このオプションを指定すると、pahcerから各ケースの実行結果がJSON形式で標準出力に出力されるようになります。これにより、Python側でのデータのパースが容易となります。

#### `--shuffle`

テストケースの実行順をシャッフルしない場合、 `WilcoxonPruner` の枝刈りにより序盤のインスタンスに過剰適合するおそれがあるため、それを避けるためにシャッフル処理を行っています。詳細は[PFNの公式ブログ](https://tech.preferred.jp/ja/blog/wilcoxonpruner/)をご参照ください。

#### `--no-result-file`

Optunaでは多数の試行を行うため、試行の度にファイル出力を行うとファイルが大量に生成されてしまいます。それを避けるため、テスト完了後にファイル出力を行わない設定としています。

#### `--freeze-best-scores`

試行の度にベスト解を更新すると、後から実行する試行の方が相対スコアを出しにくくなり不利になってしまうため、ベスト解を更新しない設定としています。
