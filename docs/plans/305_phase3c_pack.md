# Phase 3c: pack.rs ピクセルフォーマッタ

**Status**: IMPLEMENTED
**C版ファイル**: `cmspack.c`（4,062行）
**Rust見積**: ~800行（impl）+ ~400行（tests）
**ブランチ**: `feat/phase3c-pack`

## Context

Phase 5a（cnvrt.rs）でパイプライン構築が完了。end-to-end色変換（xform.rs）の前提として、
外部ピクセルバッファ ↔ パイプライン内部表現（u16/f32）の変換レイヤーが必要。

C版 cmspack.c は ~150個の特殊化フォーマッタ関数を持つが、
Rust版は汎用フォーマッタ + フラグ分岐でカバーし、頻出フォーマットのみ高速パスを用意する。

## 変更対象ファイル

| ファイル               | 操作                 |
| ---------------------- | -------------------- |
| `src/pipeline/pack.rs` | 新規作成             |
| `src/pipeline/mod.rs`  | `pub mod pack;` 追加 |

## 設計

### フォーマッタ関数型

```rust
/// 入力: バッファから1ピクセル読み出し → u16配列
/// 戻り値: 消費バイト数
pub type Formatter16In = fn(format: PixelFormat, values: &mut [u16], buf: &[u8], stride: usize) -> usize;

/// 出力: u16配列 → バッファに1ピクセル書き込み
/// 戻り値: 書き込みバイト数
pub type Formatter16Out = fn(format: PixelFormat, values: &[u16], buf: &mut [u8], stride: usize) -> usize;

/// Float入力
pub type FormatterFloatIn = fn(format: PixelFormat, values: &mut [f32], buf: &[u8], stride: usize) -> usize;

/// Float出力
pub type FormatterFloatOut = fn(format: PixelFormat, values: &[f32], buf: &mut [u8], stride: usize) -> usize;

pub enum FormatterIn {
    U16(Formatter16In),
    Float(FormatterFloatIn),
}

pub enum FormatterOut {
    U16(Formatter16Out),
    Float(FormatterFloatOut),
}
```

- `stride`: planarフォーマットでのプレーン間バイト数。chunkyでは無視
- フォーマットフラグ（swap, reverse, extra, planar, endian, premul）は `PixelFormat` から取得

### ルックアップ

```rust
pub fn find_formatter_in(format: PixelFormat, flags: u32) -> Option<FormatterIn>;
pub fn find_formatter_out(format: PixelFormat, flags: u32) -> Option<FormatterOut>;

pub const CMS_PACK_FLAGS_16BITS: u32 = 0x0000;
pub const CMS_PACK_FLAGS_FLOAT: u32 = 0x0001;
```

C版と同様のワイルドカードマスクテーブルによるマッチング。

### バイト変換ヘルパー

| 関数                               | C版マクロ           | 内容                                         |
| ---------------------------------- | ------------------- | -------------------------------------------- |
| `from_8_to_16(v: u8) -> u16`       | `FROM_8_TO_16`      | `(v as u16) << 8 \                           |
| `from_16_to_8(v: u16) -> u8`       | `FROM_16_TO_8`      | `((v as u32 * 65281 + 8388608) >> 24) as u8` |
| `reverse_flavor_8(v: u8) -> u8`    | `REVERSE_FLAVOR_8`  | `0xFF - v`                                   |
| `reverse_flavor_16(v: u16) -> u16` | `REVERSE_FLAVOR_16` | `0xFFFF - v`                                 |
| `change_endian(v: u16) -> u16`     | `CHANGE_ENDIAN`     | `v.swap_bytes()`                             |
| `lab_v2_to_v4(v: u16) -> u16`      | `FomLabV2ToLabV4`   | V2→V4エンコーディング変換                    |
| `lab_v4_to_v2(v: u16) -> u16`      | `FomLabV4ToLabV2`   | V4→V2エンコーディング変換                    |

### 実装するフォーマッタ

#### 汎用（全フラグ対応）

| 関数                  | 方向 | 内容              |
| --------------------- | ---- | ----------------- |
| `unroll_chunky_bytes` | In   | 8bit chunky 汎用  |
| `pack_chunky_bytes`   | Out  | 8bit chunky 汎用  |
| `unroll_chunky_words` | In   | 16bit chunky 汎用 |
| `pack_chunky_words`   | Out  | 16bit chunky 汎用 |
| `unroll_planar_bytes` | In   | 8bit planar 汎用  |
| `pack_planar_bytes`   | Out  | 8bit planar 汎用  |
| `unroll_planar_words` | In   | 16bit planar 汎用 |
| `pack_planar_words`   | Out  | 16bit planar 汎用 |

#### Float系

| 関数                  | 方向 | 内容    |
| --------------------- | ---- | ------- |
| `unroll_float`        | In   | f32入力 |
| `pack_float`          | Out  | f32出力 |
| `unroll_double_to_16` | In   | f64→u16 |
| `pack_double_from_16` | Out  | u16→f64 |
| `unroll_half_to_16`   | In   | f16→u16 |
| `pack_half_from_16`   | Out  | u16→f16 |

#### Lab V2

| 関数               | 方向 | 内容                 |
| ------------------ | ---- | -------------------- |
| `unroll_lab_v2_8`  | In   | LabV2 8bit → V4 u16  |
| `unroll_lab_v2_16` | In   | LabV2 16bit → V4 u16 |
| `pack_lab_v2_8`    | Out  | V4 u16 → LabV2 8bit  |
| `pack_lab_v2_16`   | Out  | V4 u16 → LabV2 16bit |

### ユーティリティ

```rust
pub fn pixel_size(format: PixelFormat) -> usize;
```

## Scope

本PRの実装範囲:

- 汎用フォーマッタ（chunky/planar、8/16bit、全フラグ対応）
- Float/Double/Half フォーマッタ
- Lab V2 フォーマッタ
- ルックアップテーブル・ファクトリ関数
- `pixel_size()`

今後のPhaseで追加:

- プラグインフォーマッタ（plugin.rs Phase）
- 特殊化高速パス（opt.rs or xform.rs Phase で必要に応じて追加）
- Premultiplied alpha（alpha.rs Phase）

## コミット構成（TDD）

### Commit 1: 計画書

### Commit 2 (RED): テスト — 変換ヘルパーと基本フォーマッタ

- `from_8_to_16` / `from_16_to_8` round-trip
- `lab_v2_to_v4` / `lab_v4_to_v2`
- RGB_8 round-trip（unroll → pack）
- GRAY_8, CMYK_16 round-trip
- `pixel_size` テスト

### Commit 3 (GREEN): 実装 — 変換ヘルパーと基本フォーマッタ

### Commit 4 (RED): テスト — フラグ・Float・Lab V2・Planar

- swap（BGR_8）、reverse（CMYK_8_REV）、extra（RGBA_8）、swapfirst（ARGB_8）
- planar（RGB_8_PLANAR、RGB_16_PLANAR）
- float（RGB_FLT、LAB_DBL）
- half-float（RGB_HALF_FLT）
- Lab V2（LABV2_8、LABV2_16）
- ルックアップ（find_formatter_in/out）

### Commit 5 (GREEN): 実装 — フラグ・Float・Lab V2・Planar

## 検証方法

```bash
cargo test pack
cargo test
cargo clippy --all-targets -- -D warnings
cargo fmt -- --check
```
