# Phase 5a: cmscnvrt.c マルチプロファイルリンク

**Status**: IMPLEMENTED
**C版ファイル**: `cmscnvrt.c`（1,226行）
**Rust見積**: ~400行（impl）+ ~200行（tests）
**ブランチ**: `feat/phase5a-cnvrt`

## Context

Phase 4d でプロファイル単体からの pipeline 構築（`read_input_lut`/`read_output_lut`）が完了。
次のステップは複数プロファイルを連結して色変換パイプラインを構築する `_cmsLinkProfiles`。
これは Phase 5b（xform.rs: Transform 構造体）の前提。

## 変更対象ファイル

| ファイル                 | 操作                                   |
| ------------------------ | -------------------------------------- |
| `src/transform/mod.rs`   | 新規モジュール宣言                     |
| `src/transform/cnvrt.rs` | パイプライン構築ロジック               |
| `src/types.rs`           | `ColorSpaceSignature::channels()` 追加 |
| `src/lib.rs`             | `transform` モジュール宣言             |

## 実装する関数

### ColorSpaceSignature ユーティリティ（types.rs）

| 関数         | C版                       | 内容                     |
| ------------ | ------------------------- | ------------------------ |
| `channels()` | `cmsChannelsOfColorSpace` | 色空間のチャネル数を返す |

### cnvrt.rs

| 関数                        | C版                       | 内容                                  |
| --------------------------- | ------------------------- | ------------------------------------- |
| `compute_absolute_intent`   | `ComputeAbsoluteIntent`   | Abs. colorimetric の行列計算          |
| `compute_bpc`               | `ComputeBlackPointComp..` | BPC 行列+オフセット計算               |
| `is_empty_layer`            | `IsEmptyLayer`            | 行列+オフセットが恒等写像か判定       |
| `compute_conversion`        | `ComputeConversion`       | プロファイル間変換レイヤ計算          |
| `add_conversion`            | `AddConversion`           | PCS 不一致ハンドリング（Lab<->XYZ）   |
| `color_space_is_compatible` | `ColorSpaceIsCompatible`  | PCS 互換性チェック                    |
| `default_icc_intents`       | `DefaultICCintents`       | 標準 ICC インテントによるリンク       |
| `link_profiles`             | `_cmsLinkProfiles`        | エントリポイント（BPC 調整+dispatch） |

## Scope（本PRでの実装範囲）

本PR では ICC 標準 4 インテントの基本パイプライン構築を実装。
以下は今後の Phase で追加予定:

- Black-preserving intents（K-only, K-plane）: xform.rs が必要（循環依存）
- Intent plugin 登録: plugin.rs Phase で実装
- BPC 実際検出（`cmsDetectBlackPoint`）: cmsgmt.rs Phase で実装
  - 本PR では BPC 計算ロジックは実装するが、黒点検出はスタブ（0,0,0 を返す）
- `cmsGetSupportedIntents`: plugin Phase で実装

## 処理フロー

### link_profiles（エントリポイント）

```text
1. nProfiles 検証（1..=255）
2. BPC 調整:
   - Abs. colorimetric → BPC=false
   - V4 + perceptual/saturation → BPC=true
3. Intent handler dispatch（本PR は DefaultICCintents のみ）
4. パイプライン返却
```

### default_icc_intents（コア）

```text
for each profile:
  1. Input/Output 判定（devicelink? first profile? PCS方向?）
  2. ColorSpace 互換性チェック
  3. Devicelink → read_devicelink_lut
     Input    → read_input_lut
     Output   → read_output_lut + ComputeConversion + AddConversion
  4. Pipeline 結合（cat）
  5. CurrentColorSpace 更新
```

### add_conversion（PCS 変換）

```text
InPCS × OutPCS の組み合わせ:
  XYZ→XYZ: matrix のみ（非恒等なら）
  XYZ→Lab: matrix + XYZ2Lab
  Lab→XYZ: Lab2XYZ + matrix
  Lab→Lab: Lab2XYZ + matrix + XYZ2Lab（非恒等なら）
  Other:   InPCS == OutPCS のみ許可
```

## コミット構成（TDD）

### Commit 1: 計画書

### Commit 2 (RED): テスト

- `color_space_is_compatible` テスト
- `is_empty_layer` テスト
- `add_conversion` テスト（XYZ->Lab, Lab->XYZ）
- `link_profiles` テスト（2-profile RGB->Lab 変換）
- `read_devicelink_lut` テスト
- `channels()` テスト

### Commit 3 (GREEN): 実装

## 検証方法

```bash
cargo test cnvrt
cargo test io
cargo test
cargo clippy --all-targets -- -D warnings
cargo fmt -- --check
```
