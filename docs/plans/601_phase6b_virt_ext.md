# Phase 6b: virt.rs 仮想プロファイル拡張（デバイスリンク・BCHSW・OkLab）

**Status**: IMPLEMENTED
**C版ファイル**: `cmsvirt.c`（1,353行）の未移植部分
**Rust見積**: ~250行（impl）+ ~200行（tests）
**ブランチ**: `feat/phase6b-virt-ext`

## Context

Phase 6a（virt.rs コア）で `new_srgb()`, `new_lab4()`, `new_rgb()` 等の基本プロファイル生成を完了。
`cmsvirt.c` の残りの関数を移植し、デバイスリンクプロファイル生成・色調調整・OkLab色空間対応を追加する。

`Transform::to_device_link()` はTransformの内部パイプラインアクセスと `FindCombination` テーブル等のインフラ変更が大きいため Phase 6c に分離する。

## 変更対象ファイル

| ファイル              | 操作                                  |
| --------------------- | ------------------------------------- |
| `src/profile/virt.rs` | 4関数追加（下記「実装する関数」参照） |

## 実装する関数

### virt.rs

| 関数                                       | C版                                   | 内容                                       |
| ------------------------------------------ | ------------------------------------- | ------------------------------------------ |
| `Profile::new_linearization_device_link()` | `cmsCreateLinearizationDeviceLinkTHR` | トーンカーブによるデバイスリンク生成       |
| `Profile::new_ink_limiting_device_link()`  | `cmsCreateInkLimitingDeviceLinkTHR`   | インクリミット制約付きCMYKデバイスリンク   |
| `Profile::new_bchsw_abstract()`            | `cmsCreateBCHSWabstractProfileTHR`    | 明度・コントラスト・色相・彩度・白色点調整 |
| `Profile::new_oklab()`                     | `cmsCreate_OkLabProfile`              | OkLab色空間プロファイル                    |

### 今後のPhaseで追加（Phase 6c）

- `Transform::to_device_link()` — `cmsTransform2DeviceLink`

## 処理フロー

### Profile::new_linearization_device_link()

```text
1. ColorSpaceから nChannels を取得
2. placeholder 生成: class=Link, cs=ColorSpace, pcs=ColorSpace
3. Pipeline(nCh→nCh) + Stage::new_tone_curves(TransferFunctions)
4. AToB0タグに書き込み
5. set_text_tags("Linearization built-in")
```

### Profile::new_ink_limiting_device_link()

```text
前提: ColorSpace == CmykData のみ対応
1. Limit を [1.0, 400.0] にクランプ
2. placeholder 生成: class=Link, cs=CMYK, pcs=CMYK
3. Pipeline(4→4):
   a) Pre: identity curves (4ch)
   b) CLUT 17^4: InkLimitingSampler で充填
   c) Post: identity curves (4ch)
4. AToB0タグに書き込み
5. set_text_tags("ink-limiting built-in")
```

### InkLimitingSampler アルゴリズム

```text
入力: CMYK 16-bit (0-65535)
出力: CMY を削減して合計インク量を制限

1. InkLimit = Limit × 655.35 (% → 16bit)
2. SumCMY = C + M + Y
3. SumCMYK = SumCMY + K
4. SumCMYK > InkLimit && SumCMY > 0:
   Ratio = 1 - (SumCMYK - InkLimit) / SumCMY
   Ratio = clamp(Ratio, 0, 1)
5. C_out = saturate(C × Ratio), M_out = saturate(M × Ratio), Y_out = saturate(Y × Ratio)
6. K_out = K (変更なし)
```

### Profile::new_bchsw_abstract()

```text
1. パラメータ: nLUTPoints, Bright, Contrast, Hue, Saturation, TempSrc, TempDest
2. TempSrc != TempDest → white_point_from_temp() で WPsrc/WPdest 算出
3. placeholder 生成: class=Abstract, cs=Lab, pcs=Lab
4. Pipeline(3→3) + CLUT nLUTPoints^3: bchswSampler で充填
5. AToB0タグに書き込み
```

### bchswSampler アルゴリズム

```text
1. Lab 16bit → float (pcs_encoded_lab_to_float)
2. Lab → LCh
3. LCh 調整:
   L = L × Contrast + Brightness
   C = C + Saturation
   h = h + Hue
4. LCh → Lab
5. 白色点調整有効時:
   Lab → XYZ (WPsrc基準) → Lab (WPdest基準)
6. Lab float → 16bit (float_to_pcs_encoded_lab)
```

### Profile::new_oklab()

```text
OkLab: 知覚的均一色空間 (XYZ/D50 ↔ OkLab via LMS)

1. 定数行列:
   - M_D50_D65, M_D65_D50 (色順応)
   - M_D65_LMS, M_LMS_D65 (LMS変換)
   - M_LMSprime_OkLab, M_OkLab_LMSprime (OkLab変換)
2. トーンカーブ: CubeRoot (γ=1/3), Cube (γ=3)
3. placeholder 生成: class=ColorSpace, cs=3colorData, pcs=XYZ
4. BToA0 パイプライン (XYZ/D50 → OkLab):
   NormFromXyzFloat → D50toD65行列 → D65toLMS行列 → CubeRoot → LMSprime→OkLab行列
5. AToB0 パイプライン (OkLab → XYZ/D50):
   OkLab→LMSprime行列 → Cube → LMStoD65行列 → D65toD50行列 → NormToXyzFloat
```

## 既存モジュール依存

| 依存先                 | 利用する関数                                                                                            |
| ---------------------- | ------------------------------------------------------------------------------------------------------- |
| `pipeline/lut.rs`      | `Pipeline`, `Stage::new_tone_curves/matrix/clut/identity_curves`, `sample_clut_16bit`, `StageLoc`, etc. |
| `curves/gamma.rs`      | `ToneCurve::build_gamma`                                                                                |
| `curves/intrp.rs`      | `quick_saturate_word`                                                                                   |
| `curves/wtpnt.rs`      | `white_point_from_temp`, `adaptation_matrix`, `d50_xyz`                                                 |
| `math/pcs.rs`          | `pcs_encoded_lab_to_float`, `float_to_pcs_encoded_lab`, `lab_to_lch`, `lch_to_lab`, `xyz_to_lab`, etc.  |
| `types.rs`             | `ColorSpaceSignature::channels()`                                                                       |
| `profile/io.rs`        | `Profile::new_placeholder`, `write_tag`, `set_version_f64`                                              |
| `profile/tag_types.rs` | `TagData::Pipeline`                                                                                     |

全て実装済み。新規外部依存なし。

## Deferred

- `Transform::to_device_link()` — Phase 6c
- `SetSeqDescTag` — ProfileSequenceDescription タグ書き込み（デバイスリンクに推奨だが必須ではない）

## コミット構成（TDD）

### Commit 1: 計画書

### Commit 2 (RED): テスト

- `new_linearization_device_link`: RGB/CMYK でヘッダ・AToB0タグ検証
- `new_ink_limiting_device_link`: CMYK limit=200% で変換結果のインク合計 ≤ 200%
- `new_ink_limiting_device_link`: 非CMYK → エラー
- `new_bchsw_abstract`: brightness=10, contrast=1.0 で L* が増加
- `new_bchsw_abstract`: hue=180 で色相回転
- `new_oklab`: ヘッダ・AToB0/BToA0タグ検証
- `new_oklab`: 白→OkLab→白 round-trip

### Commit 3 (GREEN): 実装

## 検証方法

```bash
cargo test virt
cargo test
cargo clippy --all-targets -- -D warnings
cargo fmt -- --check
```
