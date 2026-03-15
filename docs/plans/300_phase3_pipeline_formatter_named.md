# Phase 3: パイプライン・フォーマッタ・名前付きカラー（Level 2）

**Status**: PLANNED
**C版行数**: 7,116行（cmslut.c: 1,852 + cmspack.c: 4,062 + cmsnamed.c: 1,202）
**Rust見積**: ~4,000行（impl）+ ~1,800行（tests）= ~5,800行

## Context

Phase 1-2（Level 0-1）で基盤型・数学プリミティブ・カーブ・補間・色順応・CIECAM02を実装済み。Phase 3では色変換エンジンの骨格を構築する:

- **Pipeline/Stage** — 処理ステージの連結・評価（色変換パイプラインの中核）
- **Formatter** — ピクセルデータのパック/アンパック（入出力I/F）
- **MLU/NamedColor** — 多言語文字列・名前付きカラーパレット（メタデータ）

これにより後続Phase 4（プロファイルI/O）がパイプラインとフォーマッタを使えるようになる。

## 設計判断

### Stage多態性: enum（void* + 関数ポインタの代替）

C版は `void* Data` + 関数ポインタ（`EvalPtr`, `DupElemPtr`, `FreePtr`）でステージの多態性を実現。Rustでは `StageData` enumを使用:

```rust
pub enum StageData {
    Curves(Vec<ToneCurve>),
    Matrix { coefficients: Vec<f64>, offset: Option<Vec<f64>> },
    CLut(CLutData),
    NamedColor(NamedColorList),
    None, // Identity, Lab2XYZ等のデータ不要ステージ
}
```

理由: ステージ型は閉じた集合（Curves/Matrix/CLut/NamedColor/None）。enumなら `derive(Clone)` が自動、matchで網羅性チェックが効く。Phase 6でプラグイン拡張が必要になった場合は `Custom(Box<dyn StageTrait>)` バリアントを追加すればよい。

### Pipeline内部: Vec\<Stage\>（連結リストの代替）

C版は `cmsStage*` の単方向リスト。Rustでは `Vec<Stage>` を使用。パイプラインのステージ数は通常3-7個で、`insert(0)` のO(n)コストは無視できる。

### Formatter: 汎用関数 + フラグ分岐（150個の特殊化関数の代替）

C版は ~150個の個別フォーマッタ関数を持つ。Rustでは汎用フォーマッタ（chunky bytes/words、planar bytes/words）でフラグ（swap/reverse/extra/planar/endian/premul）を処理し、頻出フォーマット（RGB_8, CMYK_16等）のみ特殊化高速パスを用意する。

### MLU内部エンコーディング: UTF-16BE（ICC仕様準拠）

ICC仕様のMLUレコードはUTF-16BE。内部を `Vec<u8>`（UTF-16BEバイト列）で保持し、公開APIは `&str` / `String`（UTF-8）で入出力。Phase 4のシリアライズが単純なコピーになる。

## 実装順序（3 PR構成）

### PR 3a: `feat/phase3-named` — MLU・名前付きカラー

**ファイル**: `src/pipeline/named.rs`（新規）, `src/pipeline/mod.rs`（新規）, `src/lib.rs`（変更）

依存: `context.rs`, `types.rs` のみ。最も依存が少なく独立して実装可能。

#### 型定義

| Rust型                  | C版対応                 | 概要                                                |
| ----------------------- | ----------------------- | --------------------------------------------------- |
| `Mlu`                   | `cmsMLU`                | 多言語Unicode文字列。entries: Vec + pool: Vec\<u8\> |
| `LanguageCode([u8; 2])` | `_cmsMLUentry.Language` | ISO 639-1言語コード                                 |
| `CountryCode([u8; 2])`  | `_cmsMLUentry.Country`  | ISO 3166-1国コード                                  |
| `NamedColor`            | `_cmsNAMEDCOLOR`        | 名前 + PCS[3] + デバイス色[MAX_CHANNELS]            |
| `NamedColorList`        | `cmsNAMEDCOLORLIST`     | Vec\<NamedColor\> + prefix/suffix                   |
| `ProfileSequenceDesc`   | `cmsSEQ`                | プロファイルシーケンス記述                          |
| `Dict` / `DictEntry`    | `cmsDICT`               | メタデータ辞書                                      |

#### 公開API

```rust
impl Mlu {
    pub fn new() -> Self;
    pub fn set_ascii(&mut self, lang: &str, country: &str, text: &str) -> bool;
    pub fn set_utf8(&mut self, lang: &str, country: &str, text: &str) -> bool;
    pub fn get_ascii(&self, lang: &str, country: &str) -> Option<String>;
    pub fn get_utf8(&self, lang: &str, country: &str) -> Option<String>;
    pub fn translations_count(&self) -> usize;
    pub fn translation_codes(&self, index: usize) -> Option<(LanguageCode, CountryCode)>;
}
impl NamedColorList {
    pub fn new(colorant_count: u32, prefix: &str, suffix: &str) -> Option<Self>;
    pub fn append(&mut self, name: &str, pcs: &[u16; 3], colorant: Option<&[u16]>) -> bool;
    pub fn count(&self) -> usize;
    pub fn info(&self, index: usize) -> Option<&NamedColor>;
    pub fn find(&self, name: &str) -> Option<usize>;
}
```

#### 内部ヘルパー

- `utf8_to_utf16be(s: &str) -> Vec<u8>` — String→UTF-16BEプール書き込み
- `utf16be_to_string(bytes: &[u8]) -> String` — プール読み出し→String
- `search_entry(&self, lang, country) -> Option<usize>` — 完全一致検索
- `find_best(&self, lang, country) -> Option<usize>` — 言語フォールバック検索

#### コミット構成

| # | Type  | Commit                                                | 内容                                           |
| - | ----- | ----------------------------------------------------- | ---------------------------------------------- |
| 1 | RED   | `test(named): add MLU tests`                          | set/get ASCII/UTF8、多言語、フォールバック     |
| 2 | GREEN | `feat(named): implement MLU`                          | Mlu, LanguageCode, CountryCode, UTF-16BEプール |
| 3 | RED   | `test(named): add NamedColorList tests`               | append, find, count, 上限                      |
| 4 | GREEN | `feat(named): implement NamedColorList`               | NamedColor, NamedColorList                     |
| 5 | RED   | `test(named): add ProfileSequenceDesc and Dict tests` | alloc/clone/add                                |
| 6 | GREEN | `feat(named): implement ProfileSequenceDesc and Dict` | ProfileSequenceDesc, Dict                      |

**見積**: ~600行（impl）+ ~400行（tests）

---

### PR 3b: `feat/phase3-lut` — Pipeline・Stage

**ファイル**: `src/pipeline/lut.rs`（新規）, `src/pipeline/mod.rs`（変更）

依存: `gamma.rs`（ToneCurve）, `intrp.rs`（InterpParams）, `pcs.rs`（XYZ↔Lab）, `named.rs`（NamedColorList）

#### 型定義

| Rust型      | C版対応             | 概要                                              |
| ----------- | ------------------- | ------------------------------------------------- |
| `Stage`     | `cmsStage`          | type, implements, in/out channels, data, eval関数 |
| `StageData` | `void* Data`        | enum: Curves/Matrix/CLut/NamedColor/None          |
| `CLutData`  | `_cmsStageCLutData` | table (u16\                                       |
| `CLutTable` | `Tab` union         | enum: U16(Vec\<u16\>) / Float(Vec\<f32\>)         |
| `Pipeline`  | `cmsPipeline`       | Vec\<Stage\>, in/out channels                     |
| `StageLoc`  | `cmsStageLoc`       | enum: AtBegin / AtEnd                             |

#### 公開API

```rust
// Stage生成
impl Stage {
    pub fn new_identity(n: u32) -> Self;
    pub fn new_tone_curves(curves: Vec<ToneCurve>) -> Self;
    pub fn new_matrix(rows: u32, cols: u32, m: &[f64], offset: Option<&[f64]>) -> Option<Self>;
    pub fn new_clut_16bit(grid: &[u32], i: u32, o: u32, table: Option<&[u16]>) -> Option<Self>;
    pub fn new_clut_float(grid: &[u32], i: u32, o: u32, table: Option<&[f32]>) -> Option<Self>;
    pub fn new_lab_to_xyz() -> Self;
    pub fn new_xyz_to_lab() -> Self;
    // ... 他の特殊ステージ
    pub fn eval(&self, input: &[f32], output: &mut [f32]);
}

// Pipeline
impl Pipeline {
    pub fn new(input: u32, output: u32) -> Option<Self>;
    pub fn insert_stage(&mut self, loc: StageLoc, stage: Stage) -> bool;
    pub fn remove_stage(&mut self, loc: StageLoc) -> Option<Stage>;
    pub fn cat(&mut self, other: &Pipeline) -> bool;
    pub fn eval_16(&self, input: &[u16], output: &mut [u16]);
    pub fn eval_float(&self, input: &[f32], output: &mut [f32]);
    pub fn eval_reverse_float(&self, target: &[f32], result: &mut [f32], hint: Option<&[f32]>) -> bool;
}

// CLUTサンプリング
pub fn sample_clut_16bit<F>(stage: &mut Stage, sampler: F) -> bool;
pub fn sample_clut_float<F>(stage: &mut Stage, sampler: F) -> bool;
pub fn slice_space_16<F>(n_inputs: u32, points: &[u32], sampler: F) -> bool;
```

#### 既存モジュールの再利用

- `ToneCurve::eval_f32()` — `src/curves/gamma.rs:206` — Curvesステージ評価
- `InterpParams::eval_float()` / `eval_16()` — `src/curves/intrp.rs:125,89` — CLutステージ評価
- `InterpParams::compute()` / `compute_uniform()` — `src/curves/intrp.rs:53,40` — CLut生成時のパラメータ計算
- `pcs::xyz_to_lab()` / `lab_to_xyz()` — `src/math/pcs.rs` — Lab2XYZ/XYZ2Labステージ
- `StageSignature` — `src/types.rs:812` — ステージ型識別
- `MAX_CHANNELS` — `src/types.rs:906` — チャンネル上限

#### コミット構成

| # | Type  | Commit                                             | 内容                                       |
| - | ----- | -------------------------------------------------- | ------------------------------------------ |
| 1 | RED   | `test(lut): add Stage tests`                       | Identity, Curves, Matrixステージ           |
| 2 | GREEN | `feat(lut): implement Stage core`                  | Stage, StageData, 基本ステージ評価         |
| 3 | RED   | `test(lut): add CLUT stage tests`                  | 16bit/float CLUT生成・評価                 |
| 4 | GREEN | `feat(lut): implement CLUT stage`                  | CLutData, CLutTable, CLUT評価              |
| 5 | RED   | `test(lut): add Pipeline tests`                    | 構築、insert/remove、eval_16/eval_float    |
| 6 | GREEN | `feat(lut): implement Pipeline`                    | Pipeline全体                               |
| 7 | RED   | `test(lut): add special stages and sampling tests` | 特殊ステージ、reverse eval、sampling       |
| 8 | GREEN | `feat(lut): implement special stages and sampling` | Lab2XYZ等、sample_clut、eval_reverse_float |

**見積**: ~1,400行（impl）+ ~600行（tests）

---

### PR 3c: `feat/phase3-pack` — ピクセルフォーマッタ

**ファイル**: `src/pipeline/pack.rs`（新規）, `src/pipeline/mod.rs`（変更）

依存: `types.rs`（PixelFormat）, `math/half.rs`（half_to_float, float_to_half）

#### 型定義

| Rust型              | C版対応                     | 概要                                                  |
| ------------------- | --------------------------- | ----------------------------------------------------- |
| `Formatter16In`     | `cmsFormatter16`（入力）    | fn(&FormatterInfo, &mut [u16], &[u8], usize) -> usize |
| `Formatter16Out`    | `cmsFormatter16`（出力）    | fn(&FormatterInfo, &[u16], &mut [u8], usize) -> usize |
| `FormatterFloatIn`  | `cmsFormatterFloat`（入力） | fn(&FormatterInfo, &mut [f32], &[u8], usize) -> usize |
| `FormatterFloatOut` | `cmsFormatterFloat`（出力） | fn(&FormatterInfo, &[f32], &mut [u8], usize) -> usize |
| `FormatterIn`       | `cmsFormatter`（入力）      | enum: U16(...) / Float(...)                           |
| `FormatterOut`      | `cmsFormatter`（出力）      | enum: U16(...) / Float(...)                           |
| `FormatterInfo`     | `_cmsTRANSFORM`の一部       | input/output PixelFormat                              |

#### 公開API

```rust
pub fn find_input_formatter(format: PixelFormat, flags: u32) -> Option<FormatterIn>;
pub fn find_output_formatter(format: PixelFormat, flags: u32) -> Option<FormatterOut>;
pub fn pixel_size(format: PixelFormat) -> usize;
pub(crate) fn lab_v2_to_v4_16(x: u16) -> u16;
pub(crate) fn lab_v4_to_v2_16(x: u16) -> u16;
```

#### 実装戦略

C版の~150個の特殊化関数を以下のレイヤで置き換え:

1. **汎用フォーマッタ**（全フォーマットをカバー）
   - `unroll_chunky_bytes` / `pack_chunky_bytes` — 8bit chunky、全フラグ対応
   - `unroll_chunky_words` / `pack_chunky_words` — 16bit chunky
   - `unroll_planar_bytes` / `pack_planar_bytes` — 8bit planar
   - `unroll_planar_words` / `pack_planar_words` — 16bit planar

2. **Float/Double/Half**
   - `unroll_float` / `pack_float` — f32
   - `unroll_double` / `pack_double` — f64
   - `unroll_half` / `pack_half` — f16（half.rsを使用）

3. **Lab V2特殊処理**
   - `unroll_lab_v2_8` / `pack_lab_v2_8` — Lab V2 8bit
   - `unroll_lab_v2_16` / `pack_lab_v2_16` — Lab V2 16bit

4. **高速パス**（頻出フォーマットのみ）
   - RGB_8, BGR_8, RGBA_8, CMYK_8, GRAY_8, RGB_16, CMYK_16

#### 既存モジュールの再利用

- `PixelFormat` + アクセサ群 — `src/types.rs:109-180` — フォーマットフラグ解析
- `TYPE_*` 定数群 — `src/types.rs:186+` — 定義済みフォーマット
- `half_to_float()` / `float_to_half()` — `src/math/half.rs` — f16変換

#### コミット構成

| # | Type  | Commit                                                  | 内容                                         |
| - | ----- | ------------------------------------------------------- | -------------------------------------------- |
| 1 | RED   | `test(pack): add 8/16-bit formatter tests`              | RGB_8, CMYK_16等のround-trip                 |
| 2 | GREEN | `feat(pack): implement generic 8/16-bit formatters`     | chunky/planar byte/word                      |
| 3 | RED   | `test(pack): add format flag tests`                     | swap, reverse, extra, planar, endian, premul |
| 4 | GREEN | `feat(pack): implement format flag handling`            | 全フラグの汎用フォーマッタ                   |
| 5 | RED   | `test(pack): add float/double/half tests`               | RGB_FLT, LAB_FLT, XYZ_DBL, HALF              |
| 6 | GREEN | `feat(pack): implement float/double/half formatters`    | float系 + Lab/XYZ正規化                      |
| 7 | RED   | `test(pack): add Lab V2 and fast-path tests`            | LAB_V2_8, LAB_V2_16, 高速パス                |
| 8 | GREEN | `feat(pack): implement Lab V2 and fast-path formatters` | V2/V4変換、最適化パス                        |

**見積**: ~2,000行（impl）+ ~800行（tests）

## リスク

| リスク                                         | 対策                                                        |
| ---------------------------------------------- | ----------------------------------------------------------- |
| CLUTテーブルサイズのオーバーフロー（15D等）    | `checked_mul` で算術オーバーフロー防止、Noneを返す          |
| フォーマッタの組み合わせ爆発（~100種）         | 汎用フォーマッタで正しさを先に確保し、高速パスは後から追加  |
| 16bit↔float変換の精度                          | C版と同一の変換式（/ 65535.0, `quick_saturate_word`）を使用 |
| eval_reverse_floatのNewton法収束               | C版と同じ30回反復上限、発散時は最善結果を返す               |
| named.rsとlut.rsのNamedColorステージの循環依存 | データ構造はnamed.rs、ステージ生成はlut.rsに配置            |

## 検証方法

各PR完了時に以下を全て通過:

```bash
cargo test
cargo clippy --all-targets -- -D warnings
cargo fmt -- --check
```

Phase 3全体の完了基準:

- パイプライン構築→16bit/float評価が動作
- RGB_8、CMYK_16等の基本フォーマット変換がround-trip一致
- Stage連結・除去・結合・クローンが正しく動作
