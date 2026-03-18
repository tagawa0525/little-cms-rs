# Phase 6a: virt.rs 仮想プロファイル生成（コア）

**Status**: IMPLEMENTED
**C版ファイル**: `cmsvirt.c`（1,353行）
**Rust見積**: ~250行（impl）+ ~200行（tests）
**ブランチ**: `feat/phase6a-virt`

## Context

Phase 5b（xform.rs: Transform構造体）まで完了し、end-to-end の色変換パイプラインが動作する。
現在テストプロファイルは手動で `Profile::new_placeholder()` + タグ書き込みで構築しているが、
`cmsvirt.c` の仮想プロファイル生成関数を移植することで:

1. `Profile::new_srgb()` 等の便利APIを提供
2. `samp.rs`（黒点検出）が必要とする Lab プロファイルを生成可能にする
3. テストコードの簡素化

## 変更対象ファイル

| ファイル              | 操作                 |
| --------------------- | -------------------- |
| `src/profile/virt.rs` | 新規作成             |
| `src/profile/mod.rs`  | `pub mod virt;` 追加 |

## 実装する関数

### virt.rs

| 関数                  | C版                        | 内容                    |
| --------------------- | -------------------------- | ----------------------- |
| `Profile::new_rgb()`  | `cmsCreateRGBProfileTHR`   | RGB matrix-shaper生成   |
| `Profile::new_gray()` | `cmsCreateGrayProfileTHR`  | Grayscale生成           |
| `Profile::new_lab4()` | `cmsCreateLab4ProfileTHR`  | Lab v4 identity生成     |
| `Profile::new_lab2()` | `cmsCreateLab2ProfileTHR`  | Lab v2 identity生成     |
| `Profile::new_xyz()`  | `cmsCreateXYZProfileTHR`   | XYZ identity生成        |
| `Profile::new_srgb()` | `cmsCreate_sRGBProfileTHR` | sRGB profile生成        |
| `Profile::new_null()` | `cmsCreateNULLProfileTHR`  | NULL profile（L*抽出）  |
| `set_text_tags()`     | `SetTextTags`              | desc/copyright タグ設定 |

### 今後のPhaseで追加（Phase 6b）

- `Profile::new_linearization_device_link()` — `cmsCreateLinearizationDeviceLinkTHR`
- `Profile::new_ink_limiting_device_link()` — `cmsCreateInkLimitingDeviceLinkTHR`
- `Profile::new_bchsw_abstract()` — `cmsCreateBCHSWabstractProfileTHR`
- `Profile::new_oklab()` — `cmsCreate_OkLabProfile`
- `Transform::to_device_link()` — `cmsTransform2DeviceLink`（opt.rs依存）

## 処理フロー

### Profile::new_rgb()

```text
1. placeholder 生成
2. version=4.4, class=Display, cs=RGB, pcs=XYZ, intent=Perceptual
3. set_text_tags("RGB built-in")
4. WhitePoint指定時:
   a) D50をMediaWhitePointタグに書き込み
   b) xyY→XYZ変換
   c) adaptation_matrix(None, wp_xyz, d50) で CHAD行列計算
   d) ChromaticAdaptationタグ書き込み
5. Primaries指定時:
   a) build_rgb_to_xyz_matrix(wp, primaries) で RGB→XYZ行列
   b) 行列の列をRed/Green/Blue MatrixColumnタグに分解・書き込み
   c) Chromaticityタグ書き込み
6. TransferFunction指定時:
   a) Red/Green/Blue TRCタグ書き込み
   b) 同一カーブの場合は link_tag でリンク
```

### Profile::new_srgb()

```text
1. sRGBパラメトリックカーブ (type 4) を生成
   params = [2.4, 1/1.055, 0.055/1.055, 1/12.92, 0.04045]
2. new_rgb(D65, Rec709Primaries, [curve; 3])
3. set_text_tags("sRGB built-in") で上書き
```

### Profile::new_lab4()

```text
1. new_rgb(D50, None, None) でベースプロファイル生成
2. class=Abstract, cs=Lab, pcs=Lab に上書き
3. Pipeline(3→3) + Stage::new_identity_curves(3) を AToB0タグに書き込み
```

### Profile::new_lab2()

```text
1. new_rgb(wp, None, None) でベースプロファイル生成
2. version=2.1, class=Abstract, cs=Lab, pcs=Lab に上書き
3. Pipeline(3→3) + Stage::new_identity_clut(3) を AToB0タグに書き込み
```

### Profile::new_xyz()

```text
1. new_rgb(D50, None, None) でベースプロファイル生成
2. class=Abstract, cs=XYZ, pcs=XYZ に上書き
3. Pipeline(3→3) + Stage::new_identity_curves(3) を AToB0タグに書き込み
```

### Profile::new_null()

```text
1. placeholder 生成
2. version=4.4, class=Output, cs=Gray, pcs=Lab
3. Pipeline(3→1):
   a) PostLinearization: 3ch全ゼロカーブ
   b) Matrix [1,0,0] (1×3): L*成分を抽出
   c) OutputLinearization: 1ch全ゼロカーブ
4. BToA0タグに書き込み, D50をMediaWhitePointに書き込み
```

## 既存モジュール依存

| 依存先                 | 利用する関数                                                           |
| ---------------------- | ---------------------------------------------------------------------- |
| `curves/wtpnt.rs`      | `build_rgb_to_xyz_matrix`, `adaptation_matrix`, `d50_xyz`, `d50_xyy`   |
| `curves/gamma.rs`      | `ToneCurve::build_parametric`                                          |
| `pipeline/lut.rs`      | `Pipeline`, `Stage::new_identity_curves/clut/matrix/tone_curves`       |
| `pipeline/named.rs`    | `Mlu`                                                                  |
| `profile/io.rs`        | `Profile::new_placeholder`, `write_tag`, `link_tag`, `set_version_f64` |
| `profile/tag_types.rs` | `TagData::{Xyz,Curve,Mlu,Pipeline,S15Fixed16Array,Chromaticity}`       |
| `math/pcs.rs`          | `xyy_to_xyz`                                                           |

全て実装済み。新規外部依存なし。

## 定数

### sRGB Rec.709 パラメータ

```rust
const D65: CieXyY = CieXyY { x: 0.3127, y: 0.3290, yy: 1.0 };

const REC709_PRIMARIES: CieXyYTriple = CieXyYTriple {
    red:   CieXyY { x: 0.6400, y: 0.3300, yy: 1.0 },
    green: CieXyY { x: 0.3000, y: 0.6000, yy: 1.0 },
    blue:  CieXyY { x: 0.1500, y: 0.0600, yy: 1.0 },
};

// sRGB gamma: parametric type 4
// params = [2.4, 1/1.055, 0.055/1.055, 1/12.92, 0.04045]
```

## コミット構成（TDD）

### Commit 1: 計画書

### Commit 2 (RED): テスト

- `Profile::new_lab4()` — Lab v4 identity、save/load round-trip
- `Profile::new_lab2()` — Lab v2 identity、save/load round-trip
- `Profile::new_xyz()` — XYZ identity
- `Profile::new_srgb()` — sRGB profile、ヘッダ/タグ検証
- `Profile::new_rgb()` — カスタムRGB profile
- `Profile::new_gray()` — grayscale profile
- `Profile::new_null()` — NULL profile
- sRGB identity transform: RGB_8 round-trip（入力≒出力）
- Lab4 transform: sRGB→Lab→sRGB round-trip

### Commit 3 (GREEN): 実装

## 検証方法

```bash
cargo test virt
cargo test
cargo clippy --all-targets -- -D warnings
cargo fmt -- --check
```
