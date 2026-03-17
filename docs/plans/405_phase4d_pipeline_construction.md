# Phase 4d: cmsio1.c パイプライン構築ヘルパー

**Status**: IMPLEMENTED
**C版ファイル**: `cmsio1.c`（パイプライン構築部分）
**Rust見積**: ~500行（impl）+ ~300行（tests）
**ブランチ**: `feat/phase4d-pipeline-construction`

## Context

Phase 4c-3（LUT タグ型）がマージ済み。
プロファイルのタグから色変換パイプラインを構築する関数群を実装する。
これは Phase 5（変換エンジン: cmscnvrt.c）の前提となる。

## 変更対象ファイル

| ファイル            | 操作                                              |
| ------------------- | ------------------------------------------------- |
| `src/profile/io.rs` | パイプライン構築ヘルパー追加（Profile impl 拡張） |

## 実装する関数

### ヘルパー関数

| 関数                      | C版                       | 内容                                        |
| ------------------------- | ------------------------- | ------------------------------------------- |
| `read_media_white_point`  | `_cmsReadMediaWhitePoint` | MediaWhitePoint タグ読み取り                |
| `read_chad`               | `_cmsReadCHAD`            | ChromaticAdaptation タグ（3×3行列）読み取り |
| `read_icc_matrix_rgb2xyz` | `ReadICCMatrixRGB2XYZ`    | RGB Colorant タグから 3×3 行列構築          |

### Matrix-Shaper ビルダー（LUT 不在時のフォールバック）

| 関数                             | C版                            | 内容                                  |
| -------------------------------- | ------------------------------ | ------------------------------------- |
| `build_gray_input_pipeline`      | `BuildGrayInputMatrixPipeline` | Gray TRC → PCS（Lab or XYZ）          |
| `build_rgb_input_matrix_shaper`  | `BuildRGBInputMatrixShaper`    | RGB TRC + Colorant → XYZ（+ Lab変換） |
| `build_gray_output_pipeline`     | `BuildGrayOutputPipeline`      | PCS → Gray TRC（逆カーブ）            |
| `build_rgb_output_matrix_shaper` | `BuildRGBOutputMatrixShaper`   | XYZ → RGB（逆行列 + 逆カーブ）        |

### メイン関数（公開API）

| 関数                  | C版                     | 内容                                            |
| --------------------- | ----------------------- | ----------------------------------------------- |
| `read_input_lut`      | `_cmsReadInputLUT`      | Device→PCS パイプライン構築（AToB/DToB/matrix） |
| `read_output_lut`     | `_cmsReadOutputLUT`     | PCS→Device パイプライン構築（BToA/BToD/matrix） |
| `read_devicelink_lut` | `_cmsReadDevicelinkLUT` | Devicelink/Abstract パイプライン構築            |

### クエリ関数

| 関数                  | C版                    | 内容                           |
| --------------------- | ---------------------- | ------------------------------ |
| `is_matrix_shaper`    | `cmsIsMatrixShaper`    | Matrix-shaper プロファイル判定 |
| `is_clut`             | `cmsIsCLUT`            | CLUT ベースプロファイル判定    |
| `is_intent_supported` | `cmsIsIntentSupported` | レンダリングインテント対応判定 |

## インテント→タグ対応テーブル

```text
Intent:     0=Perceptual  1=RelColorimetric  2=Saturation  3=AbsColorimetric

AToB(16bit): AToB0         AToB1              AToB2          AToB1
DToB(float): DToB0         DToB1              DToB2          DToB3
BToA(16bit): BToA0         BToA1              BToA2          BToA1
BToD(float): BToD0         BToD1              BToD2          BToD3
```

## 処理フロー

### read_input_lut

```text
1. Float タグ (DToB) があればそれを使用（正規化ラッパー付き）
2. なければ 16bit タグ (AToB) を使用
   - インテントが見つからなければ Perceptual (AToB0) にフォールバック
3. LUT なしの場合:
   - Named Color プロファイル → NamedColor ステージ + LabV2ToV4
   - Gray プロファイル → build_gray_input_pipeline
   - RGB プロファイル → build_rgb_input_matrix_shaper
```

### read_output_lut

```text
1. Float タグ (BToD) があればそれを使用
2. なければ 16bit タグ (BToA) を使用
   - Lut16 + Lab PCS の場合は V4→V2 / V2→V4 変換ステージ挿入
3. LUT なしの場合:
   - Gray → build_gray_output_pipeline
   - RGB → build_rgb_output_matrix_shaper
```

## Lab V2↔V4 変換の挿入条件

Lut16Type（v2 レガシー）で PCS が Lab の場合:

- 入力側: パイプライン末尾に `LabV2ToV4` ステージ追加
- 出力側: パイプライン先頭に `LabV4ToV2` ステージ追加

## コミット構成（TDD）

### Commit 1-2: ヘルパー関数

RED: read_media_white_point, read_chad, read_icc_matrix_rgb2xyz テスト
GREEN: 実装

### Commit 3-4: Matrix-Shaper ビルダー

RED: gray/RGB input/output matrix-shaper テスト
GREEN: 4つのビルダー関数実装

### Commit 5-6: メイン関数 + クエリ関数

RED: read_input_lut, read_output_lut, read_devicelink_lut テスト
GREEN: インテント→タグ選択 + フォールバックロジック + クエリ関数

## 検証方法

```bash
cargo test io
cargo test
cargo clippy --all-targets -- -D warnings
cargo fmt -- --check
```
