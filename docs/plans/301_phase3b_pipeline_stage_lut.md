# Phase 3b: Pipeline・Stage（lut モジュール）

**Status**: PLANNED
**C版ファイル**: `cmslut.c`（1,852行）
**Rust見積**: ~1,400行（impl）+ ~600行（tests）
**ブランチ**: `feat/phase3-lut`

## Context

Phase 3a（MLU・NamedColorList・ProfileSequenceDesc・Dict）がマージ済み。次は色変換パイプラインの中核となる Pipeline/Stage システムを実装する。

Pipeline は Stage の連結であり、色変換の実処理を担う。後続の Phase 4（プロファイル I/O）と Phase 5（変換エンジン）はこの基盤の上に構築される。

## 変更対象ファイル

| ファイル              | 操作                |
| --------------------- | ------------------- |
| `src/pipeline/lut.rs` | 新規作成            |
| `src/pipeline/mod.rs` | `pub mod lut;` 追加 |

## 依存する既存API

| モジュール          | 使用するAPI                                                                                                                                        |
| ------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------- |
| `curves/gamma.rs`   | `ToneCurve::build_gamma()`, `build_parametric()`, `build_tabulated_16()`, `eval_f32()`                                                             |
| `curves/intrp.rs`   | `InterpParams::compute()`, `compute_uniform()`, `eval_16()`, `eval_float()`, `quick_saturate_word()`, `MAX_INPUT_DIMENSIONS`, `MAX_STAGE_CHANNELS` |
| `math/pcs.rs`       | `lab_to_xyz()`, `xyz_to_lab()`                                                                                                                     |
| `math/mtrx.rs`      | `Mat3::identity()`, `Mat3::solve()`, `Vec3::new()`                                                                                                 |
| `pipeline/named.rs` | `NamedColorList`                                                                                                                                   |
| `types.rs`          | `StageSignature`（16バリアント）, `CieXyz`, `CieLab`, `D50_X/Y/Z`, `MAX_CHANNELS`                                                                  |

## 型定義

### StageData enum

```rust
pub enum StageData {
    Curves(Vec<ToneCurve>),
    Matrix { coefficients: Vec<f64>, offset: Option<Vec<f64>> },
    CLut(CLutData),
    NamedColor(NamedColorList),
    None,
}
```

C版の `void* Data` + 関数ポインタを閉じた enum に置換。`stage_type` フィールドで eval をディスパッチ。

### CLutData / CLutTable

```rust
pub enum CLutTable {
    U16(Vec<u16>),
    Float(Vec<f32>),
}

pub struct CLutData {
    pub params: InterpParams,  // メタデータのみ（テーブル非所有）
    pub table: CLutTable,
    pub n_entries: u32,
}
```

`InterpParams` はテーブルを所有しない設計（self-reference 回避）。eval 時に `CLutTable` の参照を渡す。

### Stage

```rust
pub struct Stage {
    stage_type: StageSignature,
    implements: StageSignature,   // 最適化パターンマッチ用
    input_channels: u32,
    output_channels: u32,
    data: StageData,
}
```

### Pipeline

```rust
pub struct Pipeline {
    stages: Vec<Stage>,          // C版の連結リスト → Vec
    input_channels: u32,
    output_channels: u32,
    save_as_8bits: bool,
}
```

## C版→Rust 関数マッピング

### Stage 生成（18関数）

| C版                              | Rust メソッド                                                  |
| -------------------------------- | -------------------------------------------------------------- |
| `cmsStageAllocIdentity`          | `Stage::new_identity(n: u32)`                                  |
| `cmsStageAllocToneCurves`        | `Stage::new_tone_curves(curves: Option<&[ToneCurve]>, n: u32)` |
| `_cmsStageAllocIdentityCurves`   | `Stage::new_identity_curves(n: u32)`                           |
| `cmsStageAllocMatrix`            | `Stage::new_matrix(rows, cols, matrix, offset)`                |
| `cmsStageAllocCLut16bit`         | `Stage::new_clut_16bit_uniform(grid, in, out, table)`          |
| `cmsStageAllocCLut16bitGranular` | `Stage::new_clut_16bit(grid, in, out, table)`                  |
| `cmsStageAllocCLutFloat`         | `Stage::new_clut_float_uniform(grid, in, out, table)`          |
| `cmsStageAllocCLutFloatGranular` | `Stage::new_clut_float(grid, in, out, table)`                  |
| `_cmsStageAllocIdentityCLut`     | `Stage::new_identity_clut(n: u32)`                             |
| `_cmsStageAllocLab2XYZ`          | `Stage::new_lab_to_xyz()`                                      |
| `_cmsStageAllocXYZ2Lab`          | `Stage::new_xyz_to_lab()`                                      |
| `_cmsStageAllocLabV2ToV4curves`  | `Stage::new_lab_v2_to_v4_curves()`                             |
| `_cmsStageAllocLabV2ToV4`        | `Stage::new_lab_v2_to_v4()`                                    |
| `_cmsStageAllocLabV4ToV2`        | `Stage::new_lab_v4_to_v2()`                                    |
| `_cmsStageNormalizeFromLabFloat` | `Stage::new_normalize_from_lab_float()`                        |
| `_cmsStageNormalizeToLabFloat`   | `Stage::new_normalize_to_lab_float()`                          |
| `_cmsStageNormalizeFromXyzFloat` | `Stage::new_normalize_from_xyz_float()`                        |
| `_cmsStageNormalizeToXyzFloat`   | `Stage::new_normalize_to_xyz_float()`                          |
| `_cmsStageClipNegatives`         | `Stage::new_clip_negatives(n: u32)`                            |
| `_cmsStageAllocLabPrelin`        | `Stage::new_lab_prelin()`                                      |

### Stage アクセサ・ライフサイクル

| C版                                                | Rust                                                 |
| -------------------------------------------------- | ---------------------------------------------------- |
| `cmsStageInputChannels` / `cmsStageOutputChannels` | `stage.input_channels()` / `stage.output_channels()` |
| `cmsStageType`                                     | `stage.stage_type()`                                 |
| `cmsStageData`                                     | `stage.data()`                                       |
| `cmsStageDup` / `cmsStageFree`                     | `Clone` / `Drop`（自動）                             |
| `_cmsStageGetPtrToCurveSet`                        | `stage.curves()`                                     |

### Pipeline（12関数）

| C版                                      | Rust                                                    |
| ---------------------------------------- | ------------------------------------------------------- |
| `cmsPipelineAlloc`                       | `Pipeline::new(input, output)`                          |
| `cmsPipelineDup` / `cmsPipelineFree`     | `Clone` / `Drop`                                        |
| `cmsPipelineInsertStage`                 | `pipeline.insert_stage(loc, stage)`                     |
| `cmsPipelineUnlinkStage`                 | `pipeline.remove_stage(loc)`                            |
| `cmsPipelineCat`                         | `pipeline.cat(other)`                                   |
| `cmsPipelineEval16`                      | `pipeline.eval_16(input, output)`                       |
| `cmsPipelineEvalFloat`                   | `pipeline.eval_float(input, output)`                    |
| `cmsPipelineEvalReverseFloat`            | `pipeline.eval_reverse_float(target, result, hint)`     |
| `cmsPipelineStageCount`                  | `pipeline.stage_count()`                                |
| `cmsPipelineGetPtrToFirstStage` / `Last` | `pipeline.first_stage()` / `last_stage()`               |
| `cmsPipelineSetSaveAs8bitsFlag`          | `pipeline.set_save_as_8bits(on)`                        |
| `cmsPipelineCheckAndRetreiveStages`      | `pipeline.check_and_retrieve_stages(&[StageSignature])` |

### サンプリング（4関数）

| C版                       | Rust                                           |
| ------------------------- | ---------------------------------------------- |
| `cmsStageSampleCLut16bit` | `sample_clut_16bit(stage, sampler, flags)`     |
| `cmsStageSampleCLutFloat` | `sample_clut_float(stage, sampler, flags)`     |
| `cmsSliceSpace16`         | `slice_space_16(n_inputs, points, sampler)`    |
| `cmsSliceSpaceFloat`      | `slice_space_float(n_inputs, points, sampler)` |

### ユーティリティ

| C版               | Rust                                                          |
| ----------------- | ------------------------------------------------------------- |
| `_cmsQuantizeVal` | `pub(crate) fn quantize_val(i: f64, max_samples: u32) -> u16` |
| `FromFloatTo16`   | `fn float_to_16(input, output)` — private                     |
| `From16ToFloat`   | `fn from_16_to_float(input, output)` — private                |
| `CubeSize`        | `fn cube_size(dims, n) -> Option<u32>` — checked arithmetic   |

## eval ディスパッチ

`Stage::eval` は `stage_type` で分岐。特殊ステージ（LabV2toV4 等）は `stage_type = MatrixElem` で生成され、`eval_matrix` が自然に適用される。`implements` はパターンマッチ用であり eval 分岐には使わない。

| stage_type          | 評価関数                                                                     |
| ------------------- | ---------------------------------------------------------------------------- |
| `IdentityElem`      | 入力をそのままコピー                                                         |
| `CurveSetElem`      | 各チャンネルに `ToneCurve::eval_f32`                                         |
| `MatrixElem`        | 行列積 + オフセット（f64 精度中間値）                                        |
| `CLutElem`          | Float テーブル → `InterpParams::eval_float`、U16 テーブル → f32↔u16 変換経由 |
| `Lab2XyzElem`       | Lab 正規化復元 → `pcs::lab_to_xyz` → XYZ 正規化                              |
| `Xyz2LabElem`       | XYZ 正規化復元 → `pcs::xyz_to_lab` → Lab 正規化                              |
| `ClipNegativesElem` | `max(0, x)`                                                                  |

## Pipeline 評価

### eval_float

ダブルバッファ（ping-pong）パターン。`[f32; MAX_STAGE_CHANNELS]` × 2 のスタックバッファで Stage を順次評価。

### eval_16

入力 u16→f32 変換 → `eval_float` と同等のステージ評価 → 出力 f32→u16 変換。

### eval_reverse_float（Newton 法）

3→3 または 4→3 パイプライン限定。ヤコビアン 3×3 を数値微分で構築し、`Mat3::solve` で連立方程式を解く。最大30回反復、誤差増加時に打ち切り。

## コミット構成（TDD）

### Commit 1: RED — Stage 基本テスト

```text
test(lut): add Stage tests for Identity, Curves, and Matrix
```

- `stage_identity_passthrough`: 3ch Identity 入出力一致
- `stage_identity_channel_count`: チャンネル数確認
- `stage_curves_gamma`: gamma(2.2) 評価
- `stage_curves_per_channel`: チャンネル別カーブ
- `stage_matrix_identity`: 単位行列パススルー
- `stage_matrix_scale`: スケーリング行列
- `stage_matrix_with_offset`: オフセット付き行列
- `stage_matrix_invalid_dims`: 不正次元 → None
- `stage_clone`: Clone の独立性

### Commit 2: GREEN — Stage コア実装

```text
feat(lut): implement Stage core (Identity, Curves, Matrix)
```

`Stage`, `StageData`, `new_identity`, `new_tone_curves`, `new_identity_curves`, `new_matrix`, `eval` ディスパッチ、アクセサ、`float_to_16`/`from_16_to_float`/`quantize_val`

### Commit 3: RED — CLUT テスト

```text
test(lut): add CLUT stage tests
```

- `clut_16bit_identity`: 2点均一グリッド Identity
- `clut_16bit_interpolation`: 17点グリッド中間値補間
- `clut_float_identity`: Float 版 Identity
- `clut_granular_grid`: 不均一グリッド
- `clut_table_none_zeros`: None テーブル → ゼロ初期化
- `clut_identity_clut`: `new_identity_clut` 動作
- `clut_too_many_inputs`: >15 入力 → None
- `clut_clone`: Clone 独立性

### Commit 4: GREEN — CLUT 実装

```text
feat(lut): implement CLUT stage
```

`CLutTable`, `CLutData`, `new_clut_16bit(_uniform)`, `new_clut_float(_uniform)`, `new_identity_clut`, `eval_clut_float`, `eval_clut_16_in_float`, `cube_size`

### Commit 5: RED — Pipeline テスト

```text
test(lut): add Pipeline tests
```

- `pipeline_empty`: 空パイプライン
- `pipeline_insert_at_end` / `at_begin`: 挿入順序
- `pipeline_remove`: 先頭・末尾削除
- `pipeline_eval_float_single_stage`: 単一ステージ float
- `pipeline_eval_float_chain`: curves→matrix→curves チェーン
- `pipeline_eval_16`: 16bit パス
- `pipeline_cat`: 結合
- `pipeline_clone`: Clone 一致
- `pipeline_check_and_retrieve_stages`: パターンマッチ

### Commit 6: GREEN — Pipeline 実装

```text
feat(lut): implement Pipeline
```

`Pipeline`, `new`, `insert_stage`, `remove_stage`, `cat`, `eval_float`, `eval_16`, アクセサ群、`bless_pipeline`

### Commit 7: RED — 特殊ステージ・サンプリング・逆変換テスト

```text
test(lut): add special stages, sampling, and reverse eval tests
```

- `stage_lab_to_xyz` / `xyz_to_lab`: D50 白色点往復
- `stage_clip_negatives`: 負値クリップ
- `stage_lab_v2_v4_roundtrip`: V2↔V4 往復
- `stage_normalize_lab_float`: Lab 正規化往復
- `stage_normalize_xyz_float`: XYZ 正規化往復
- `sample_clut_16bit_identity` / `inspect`: サンプリング
- `slice_space_16`: 全ノード走査
- `pipeline_eval_reverse_float`: Identity 逆変換
- `pipeline_eval_reverse_3to3`: 既知変換の逆変換
- `pipeline_eval_reverse_wrong_dims`: 不正次元 → false

### Commit 8: GREEN — 特殊ステージ・サンプリング・逆変換実装

```text
feat(lut): implement special stages, sampling, and reverse eval
```

`new_lab_to_xyz`, `new_xyz_to_lab`, `new_clip_negatives`, Lab V2/V4 変換群, Lab/XYZ 正規化群, `new_lab_prelin`, `sample_clut_16bit`, `sample_clut_float`, `slice_space_16`, `slice_space_float`, `eval_reverse_float`

## エッジケース・エラー処理

- **CLUT テーブルサイズ**: `cube_size` は `checked_mul` でオーバーフロー防止、`None` を返す
- **チャンネル上限**: `MAX_CHANNELS`(16), `MAX_INPUT_DIMENSIONS`(15), `MAX_STAGE_CHANNELS`(128)
- **Pipeline 整合性**: `bless_pipeline` で隣接ステージの入出力チャンネル一致を検証
- **空パイプライン**: eval 時は入力をそのままコピー
- **逆変換の発散**: 誤差増加時に打ち切り、最善結果を返す。ヤコビアン特異時は `false`
- **None カーブ**: `new_tone_curves(None, n)` は identity gamma(1.0) で生成

## 検証方法

```bash
cargo test lut              # lut モジュールテスト
cargo test                  # 全テスト（回帰確認）
cargo clippy --all-targets -- -D warnings
cargo fmt -- --check
```
