# Phase 5c: samp.rs 黒点検出

**Status**: IMPLEMENTED
**C版ファイル**: `cmssamp.c`（599行）
**Rust見積**: ~250行（impl）+ ~150行（tests）
**ブランチ**: `feat/phase5c-samp`

## Context

Phase 5b（xform.rs: Transform）と Phase 6a（virt.rs: Lab/sRGB プロファイル生成）が完了。
`cnvrt.rs` の `compute_conversion()` に BPC（Black Point Compensation）のスタブが残っている。
`cmssamp.c` は黒点検出アルゴリズムを実装し、BPC に必要な黒点座標を計算する。

## 変更対象ファイル

| ファイル | 操作 |
| --- | --- |
| `src/transform/samp.rs` | 新規作成: 黒点検出 |
| `src/transform/mod.rs` | `pub mod samp;` 追加 |
| `src/transform/cnvrt.rs` | BPC スタブ解消: `compute_conversion` で黒点検出実行 |
| `src/profile/io.rs` | `is_intent_supported()` ヘルパー追加 |
| `src/math/pcs.rs` | `endpoints_by_space()` ヘルパー追加 |

## 実装する型・関数

### samp.rs

| 関数 | C版 | 内容 |
| --- | --- | --- |
| `detect_black_point()` | `cmsDetectBlackPoint` | 入力プロファイルの黒点検出 |
| `detect_dest_black_point()` | `cmsDetectDestinationBlackPoint` | 出力プロファイルの黒点検出(Adobe) |
| `black_point_as_darker_colorant()` | `BlackPointAsDarkerColorant` | 最暗色素による黒点検出 |
| `black_point_using_perceptual()` | `BlackPointUsingPerceptualBlack` | Perceptual往復による黒点検出 |
| `is_ink_colorspace()` | `isInkColorspace` | インク系色空間判定 |

### ヘルパー（他モジュール追加）

| 関数 | C版 | 配置先 |
| --- | --- | --- |
| `endpoints_by_space()` | `_cmsEndPointsBySpace` | `math/pcs.rs` |
| `formatter_for_colorspace()` | `cmsFormatterForColorspaceOfProfile` | `pipeline/pack.rs` |
| `Profile::is_intent_supported()` | `cmsIsIntentSupported` | `profile/io.rs` |

### cnvrt.rs 変更

`compute_conversion()` の `_bpc` パラメータを有効化し、BPC が true の場合に
`detect_black_point()` / `detect_dest_black_point()` を呼んで `compute_bpc()` を適用。

## 処理フロー

### detect_black_point() — 入力/デバイスリンク

```text
1. プロファイルクラス検証（Link/Abstract/NamedColor → 失敗）
2. インテント検証（Absolute colorimetric → 失敗）
3. V4 + Perceptual/Saturation:
   a) matrix-shaper → black_point_as_darker_colorant()
   b) 非matrix-shaper → PERCEPTUAL_BLACK 定数返却
4. V2 + Relative colorimetric:
   → black_point_as_darker_colorant()
5. Output class:
   → black_point_using_perceptual()
6. その他:
   → black_point_as_darker_colorant()
```

### detect_dest_black_point() — 出力プロファイル (Adobe アルゴリズム)

```text
1. 基本チェック（detect_black_point と同じ）
2. V4 Perceptual/Saturation → 上記と同じ
3. LUT ベース + ink colorspace:
   a) 初期黒点計算
   b) Lab ラウンドトリップ変換生成
   c) 256 点 L* ランプ生成 + 単調性強制
   d) 中間域の直線性テスト
   e) 非直線 → 最小二乗二次曲線フィッティング
4. フォールバック: detect_black_point() に委譲
```

### black_point_as_darker_colorant()

```text
1. endpoints_by_space() で最暗色素取得
2. Profile→Lab 変換を作成
3. 最暗色素を Lab に変換
4. L* を [0, 50] にクリップ、a=b=0 に強制
5. Lab→XYZ 変換して返却
```

## 定数

```rust
pub const PERCEPTUAL_BLACK_X: f64 = 0.00336;
pub const PERCEPTUAL_BLACK_Y: f64 = 0.0034731;
pub const PERCEPTUAL_BLACK_Z: f64 = 0.00287;
```

## 既存モジュール依存

| 依存先 | 利用する関数 |
| --- | --- |
| `profile/virt.rs` | `Profile::new_lab4()`, `Profile::new_lab2()` |
| `transform/xform.rs` | `Transform::new()`, `Transform::new_multiprofile()` |
| `transform/cnvrt.rs` | `compute_bpc()`（既存） |
| `math/pcs.rs` | `xyz_to_lab()`, `lab_to_xyz()` |
| `math/mtrx.rs` | `Mat3::solve()` |
| `curves/wtpnt.rs` | `adapt_to_illuminant()` |
| `profile/io.rs` | `is_matrix_shaper()`, `has_tag()` |
| `types.rs` | `ColorSpaceSignature::channels()`, `to_pixel_type()` |

## コミット構成（TDD）

### Commit 1: 計画書

### Commit 2 (RED): テスト

- `detect_black_point()` sRGB → 黒点 ≈ (0, 0, 0)
- `detect_black_point()` V4 perceptual 非matrix-shaper → PERCEPTUAL_BLACK
- `detect_dest_black_point()` sRGB → 黒点 L* ∈ [0, 5]
- `is_ink_colorspace()` 判定テスト
- `endpoints_by_space()` RGB/CMYK/Lab テスト
- BPC 有効時の Transform round-trip テスト

### Commit 3 (GREEN): 実装

ヘルパー追加 → samp.rs 実装 → cnvrt.rs BPC 統合

## 検証方法

```bash
cargo test samp
cargo test cnvrt
cargo test
cargo clippy --all-targets -- -D warnings
cargo fmt -- --check
```
