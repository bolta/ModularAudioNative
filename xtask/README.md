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
# release ビルド（デフォルト）。dist/ に成果物が生成される
cargo xtask build

# debug ビルド
cargo xtask build --profile debug

# 出力先を変更
cargo xtask build --out-dir path/to/output
```

`cargo xtask` は `.cargo/config.toml` に定義したエイリアスで、実体は
`cargo run --package xtask -- <引数>` です。エイリアスを使わず直接
`cargo run -p xtask -- build --profile release` のように呼び出しても構いません。

## オプション

| オプション | デフォルト | 説明 |
| --- | --- | --- |
| `--profile <debug\|release>` | `release` | `cargo build` / `dx build` に渡すビルドプロファイル |
| `--out-dir <path>` | `dist`（ワークスペースルート直下） | 成果物のコピー先ディレクトリ |

## 生成される内容

```
dist/
├─ moddl.exe              # main を cargo build した実行ファイル
├─ gui_controller.exe     # gui_controller を dx build した実行ファイル
├─ assets/                # gui_controller.exe が実行時に隣接して必要とする資産（favicon, css など）
├─ portaudio_x64.dll      # main/res/ からコピー
└─ builtins/
   └─ root.moddl          # main/res/builtins/ からコピー
```

`assets/` は dioxus desktop がビルド時に exe と同階層に出力するファイル群で、
実行時に必要なため削除せずそのまま配置しています。

## 内部でやっていること

1. `cargo build --package moddl [--release]` をワークスペースルートで実行
2. `dx build [--release]` を `gui_controller/` ディレクトリで実行
3. `target/release(or debug)/moddl.exe` と
   `target/dx/gui_controller/release(or debug)/windows/app/` 以下の成果物、
   `main/res/` 以下のファイルを `--out-dir` へコピー

## CI での利用

[.github/workflows/rust.yml](../.github/workflows/rust.yml) では、`dioxus-cli` を
インストールした後に `cargo xtask build --profile release` を実行し、`dist/` を
そのままビルドアーティファクトとしてアップロードしています。
