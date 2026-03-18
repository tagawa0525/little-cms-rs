# Phase 5f: alpha.rs アルファチャネル処理

**Status**: PLANNED
**C版ファイル**: `cmsalpha.c`（650行）
**Rust見積**: ~200行（impl）+ ~100行（tests）
**ブランチ**: `feat/phase5f-alpha`

## Context

Phase 5e（gmt.rs: ガマットマッピング）完了。Phase 5 の最後のモジュール。
`cmsalpha.c` はカラー変換時にアルファ（エクストラ）チャネルをコピーする。
`FLAGS_COPY_ALPHA` が設定されている場合、変換の色チャネルとは独立に
入力の余剰チャネルを出力にコピーする。

## 変更対象ファイル

| ファイル                 | 操作                                                   |
| ------------------------ | ------------------------------------------------------ |
| `src/transform/alpha.rs` | 新規: アルファチャネルコピー                           |
| `src/transform/mod.rs`   | `pub mod alpha;` 追加                                  |
| `src/transform/xform.rs` | `FLAGS_COPY_ALPHA` 定数追加、`do_transform` 内呼び出し |

## 実装する関数

### alpha.rs

| 関数                      | C版                       | 内容                                         |
| ------------------------- | ------------------------- | -------------------------------------------- |
| `handle_extra_channels()` | `_cmsHandleExtraChannels` | エクストラチャネルのコピー（メインエントリ） |

### xform.rs 変更

- `FLAGS_COPY_ALPHA` 定数追加（0x04000000）
- `do_transform_16` / `do_transform_float` でアルファコピー呼び出し

## 処理フロー

### handle_extra_channels()

```text
1. FLAGS_COPY_ALPHA 未設定 → return
2. 入力と出力のextraチャネル数が異なる → return
3. extraチャネル数 == 0 → return
4. 入出力フォーマットからチャネルあたりのバイト数を取得
5. 各ピクセルのエクストラチャネルをコピー:
   a) 色チャネル分のオフセットをスキップ
   b) extra チャネルを入力バイト幅で読み、出力バイト幅に変換して書き込み
```

### フォーマット変換

入出力のバイト幅が異なる場合の変換:

| 入力 → 出力  | 変換方法                          |
| ------------ | --------------------------------- |
| 8bit → 16bit | `v as u16 * 257` (0xFF → 0xFFFF)  |
| 16bit → 8bit | `(v + 128) / 257` (0xFFFF → 0xFF) |
| 同一幅       | バイトコピー                      |

float変換はスコープ外（chunky int のみ初回実装）。

## Deferred

- Planar レイアウト対応
- Half-float / double / float 間のエクストラチャネル変換
- Swap-endian 16bit エクストラチャネル

## コミット構成（TDD）

### Commit 1: 計画書

### Commit 2 (RED): テスト

- RGBA_8 → RGBA_8: アルファチャネルがコピーされる
- RGBA_8 → RGB_8: extra 数不一致、アルファ無視
- RGB_8 → RGB_8: extra なし、正常動作
- FLAGS_COPY_ALPHA なし: アルファコピーされない
- RGBA_16 → RGBA_8: 16→8bit 変換

### Commit 3 (GREEN): 実装

## 検証方法

```bash
cargo test alpha
cargo test xform
cargo test
cargo clippy --all-targets -- -D warnings
cargo fmt -- --check
```
