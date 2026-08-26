# xtask

`moddl.exe` / `gui_controller.exe` / `portaudio_x64.dll` / `builtins/root.moddl` を
まとめてビルドし、1か所（デフォルトは `dist/`）に集めるためのビルドオーケストレーターです。

`cargo` だけでは `main` (moddl) のビルドと `gui_controller` (dioxus) のビルドを一度に
行えないため、この2つを起動してから成果物をコピーする処理を Rust コードとして
`xtask` クレートにまとめています（[cargo-xtask パターン](https://github.com/matklad/cargo-xtask)）。
bash や PowerShell のような外部シェルに依存しないので、手元の WSL2 でも
GitHub Actions (windows-latest) でも同じコマンドで同じ手順が実行されます。

## 前提

- `cargo`（main / gui_controller のビルドに使用）
- [`dx`](https://dioxuslabs.com/learn/0.7/getting_started)（dioxus-cli。`gui_controller` のビルドに使用）
  - `gui_controller/Cargo.toml` の `dioxus` バージョンとおおむね揃えてインストールしてください
    （例: `cargo install dioxus-cli --version "0.7.10" --locked`）

## 使い方

ワークスペースルートで実行します。

```sh
# release ビルド（デフォルト）。dist/release/ に成果物が生成される
cargo xtask build

# debug ビルド。dist/debug/ に成果物が生成される
cargo xtask build --profile debug

# 出力先を変更（この場合 path/to/output に直接出力される。profile のサブディレクトリは作られない）
cargo xtask build --out-dir path/to/output
```

`cargo xtask` は `.cargo/config.toml` に定義したエイリアスで、実体は
`cargo run --package xtask -- <引数>` です。エイリアスを使わず直接
`cargo run -p xtask -- build --profile release` のように呼び出しても構いません。

## オプション

| オプション | デフォルト | 説明 |
| --- | --- | --- |
| `--profile <debug\|release>` | `release` | `cargo build` / `dx build` に渡すビルドプロファイル |
| `--out-dir <path>` | `dist/<profile>`（ワークスペースルート直下） | 成果物のコピー先ディレクトリ |

`--out-dir` を省略した場合、debug ビルドと release ビルドの成果物を取り違えないよう
`dist/debug/` と `dist/release/` に分かれます。この既定の出力先はビルドのたびに
**まるごと削除してから作り直す**ため、前回ビルドの古い成果物が混ざることはありません。

`--out-dir` を明示的に指定した場合は、profile のサブディレクトリを挟まずそのディレクトリに
直接出力します。素性の分からないディレクトリを無条件に削除するのは危険なため、この場合は
既存の中身を削除せず、上書きコピーのみ行います。

## 生成される内容

```
dist/
├─ debug/                 # --profile debug の場合の出力先
└─ release/               # --profile release の場合の出力先（デフォルト）
   ├─ moddl.exe              # main を cargo build した実行ファイル
   ├─ gui_controller.exe     # gui_controller を dx build した実行ファイル
   ├─ assets/                # gui_controller.exe が実行時に隣接して必要とする資産（favicon, css など）
   ├─ portaudio_x64.dll      # main/res/ からコピー
   └─ builtins/
      └─ root.moddl          # main/res/builtins/ からコピー
```

（`--out-dir` を指定した場合は `debug/`・`release/` を挟まず、指定したディレクトリ直下に
上記と同じ内容が生成されます。）

`assets/` は dioxus desktop がビルド時に exe と同階層に出力するファイル群で、
実行時に必要なため削除せずそのまま配置しています。

## 内部でやっていること

1. `cargo build --package moddl [--release]` をワークスペースルートで実行
2. `dx build [--release]` を `gui_controller/` ディレクトリで実行
3. `--out-dir` が未指定なら `dist/<profile>/` を出力先とし、既存のディレクトリが
   あれば削除してから作り直す（`--out-dir` を指定した場合はこの削除は行わない）
4. `target/release(or debug)/moddl.exe` と
   `target/dx/gui_controller/release(or debug)/windows/app/` 以下の成果物、
   `main/res/` 以下のファイルを出力先へコピー

## CI での利用

[.github/workflows/rust.yml](../.github/workflows/rust.yml) では、`dioxus-cli` を
インストールした後に `cargo xtask build --profile release` を実行し、`dist/release/` を
そのままビルドアーティファクトとしてアップロードしています。
