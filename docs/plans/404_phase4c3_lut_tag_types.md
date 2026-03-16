# Phase 4c-3: LUT タグ型ハンドラ

**Status**: IMPLEMENTED
**C版ファイル**: `cmstypes.c`（LUT部分）
**Rust見積**: ~600行（impl）+ ~300行（tests）
**ブランチ**: `feat/phase4c3-lut-tags`

## Context

Phase 4c-2（ProfileSequenceDesc/Id, vcgt, Dict）がマージ済み。
ICC プロファイルの色変換パイプラインを格納する LUT タグ型を実装する。

## 変更対象ファイル

| ファイル                   | 操作                                  |
| -------------------------- | ------------------------------------- |
| `src/profile/tag_types.rs` | LUT タグ型ハンドラ追加、dispatch 拡張 |

## 実装するタグ型

| TagTypeSignature | C版関数                | 内容                                 |
| ---------------- | ---------------------- | ------------------------------------ |
| Lut8             | Type_LUT8_Read/Write   | 8bit LUT（matrix + curves + CLUT）   |
| Lut16            | Type_LUT16_Read/Write  | 16bit LUT（可変長テーブル）          |
| LutAtoB          | Type_LUTA2B_Read/Write | V4 AtoB パイプライン（offset-based） |
| LutBtoA          | Type_LUTB2A_Read/Write | V4 BtoA パイプライン（offset-based） |

## データフォーマット概要

### Lut8/Lut16（legacy v2）

```text
Header: inputCh(u8) outputCh(u8) clutPoints(u8) pad(u8) [+ inputEntries(u16) outputEntries(u16) for Lut16]
Matrix: 3×3 s15Fixed16（inputCh==3 のとき適用）
Input curves: 256 entries × inputCh (Lut8=u8, Lut16=u16×inputEntries)
CLUT: clutPoints^inputCh × outputCh entries
Output curves: 256 entries × outputCh (Lut8=u8, Lut16=u16×outputEntries)
```

### LutAtoB/LutBtoA（v4）

```text
Header: inputCh(u8) outputCh(u8) pad(u16) offsetB(u32) offsetMat(u32) offsetM(u32) offsetCLUT(u32) offsetA(u32)
```

- AtoB 順序: A curves → CLUT → M curves → Matrix → B curves
- BtoA 順序: B curves → Matrix → M curves → CLUT → A curves

各要素は offset が 0 なら不在。offset は tag base からの相対位置。

## TagData の拡張

```rust
Pipeline(Pipeline),  // 新規バリアント
```

## ヘルパー関数

| 関数                 | 用途                                        |
| -------------------- | ------------------------------------------- |
| `from_8_to_16`       | u8 → u16 変換（v * 257）                    |
| `from_16_to_8`       | u16 → u8 変換                               |
| `uipow`              | オーバーフロー安全な累乗（CLUT サイズ計算） |
| `read_8bit_tables`   | 8bit カーブテーブル読み込み                 |
| `write_8bit_tables`  | 8bit カーブテーブル書き込み                 |
| `read_16bit_tables`  | 16bit カーブテーブル読み込み                |
| `write_16bit_tables` | 16bit カーブテーブル書き込み                |
| `read_v4_curve_set`  | V4 埋め込みカーブセット読み込み             |
| `write_v4_curve_set` | V4 埋め込みカーブセット書き込み             |
| `read_v4_clut`       | V4 CLUT 読み込み（per-dimension grid）      |
| `write_v4_clut`      | V4 CLUT 書き込み                            |
| `read_v4_matrix`     | V4 行列 + オフセット読み込み                |
| `write_v4_matrix`    | V4 行列 + オフセット書き込み                |

## コミット構成（TDD）

### Commit 1-2: Lut8

RED: Lut8 roundtrip テスト（matrix + curves + CLUT）
GREEN: read/write + 8bit ヘルパー

### Commit 3-4: Lut16

RED: Lut16 roundtrip テスト
GREEN: read/write + 16bit ヘルパー

### Commit 5-6: LutAtoB + LutBtoA

RED: AtoB/BtoA roundtrip テスト
GREEN: read/write + V4 ヘルパー + dispatch

## 検証方法

```bash
cargo test tag_types
cargo test
cargo clippy --all-targets -- -D warnings
cargo fmt -- --check
```
