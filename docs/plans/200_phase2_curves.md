# Phase 2: カーブ・補間・色順応・CIECAM02

- **Status**: PLANNED
- **Phase**: 2 (Level 1)
- **C版行数**: 3,687行
- **Rust概算**: ~3,500行
- **前提**: Phase 1完了（types, context, math/mtrx, math/pcs）
- **ブランチ**: `feat/phase2-curves`

## Context

Phase 1で実装した基盤型（CieXyz/CieLab/固定小数点/Mat3/Vec3等）の上に、色変換パイプラインの中核となるカーブ・補間エンジンを構築する。トーンカーブ（ガンマ補正）はICCプロファイルの基本構成要素であり、LUT補間は多次元カラーテーブルの評価に不可欠。白色点変換とCIECAM02は色順応と知覚的色変換に必要。

## モジュール一覧と実装順序

全体戦略計画書に従い、`src/curves/` ディレクトリに配置する。

```text
intrp（独立）──→ gamma（intrpに依存）
wtpnt（math/pcs, math/mtrxに依存）
cam02（独立）
```

| # | モジュール | C版 | 行数 | 内容 |
| - | -------------------- | -------------- | ----- | ------------------------------------------ |
| 1 | `src/curves/intrp.rs` | `cmsintrp.c` | 1,330 | 1D-15D LUT補間（16bit/float双方パス） |
| 2 | `src/curves/gamma.rs` | `cmsgamma.c` | 1,514 | トーンカーブ（パラメトリック・テーブル等） |
| 3 | `src/curves/wtpnt.rs` | `cmswtpnt.c` | 353 | 白色点・色温度・Bradford色順応 |
| 4 | `src/curves/cam02.rs` | `cmscam02.c` | 490 | CIECAM02順方向・逆方向変換 |

---

### 1. `src/curves/intrp.rs` — LUT補間

**C版対応**: `cmsintrp.c`（1,330行）

#### 設計判断: InterpParamsはテーブルを所有しない

C版では`InterpParams.Table`が生データへの生ポインタを保持する。Rustでは自己参照を避けるため、`InterpParams`はメタデータ（grid寸法・stride等）のみを保持し、テーブルは評価時に引数として渡す。

```rust
pub const MAX_INPUT_DIMENSIONS: usize = 15;

// 補間フラグ
pub const LERP_FLAGS_16BITS: u32 = 0x0000;
pub const LERP_FLAGS_FLOAT: u32 = 0x0001;
pub const LERP_FLAGS_TRILINEAR: u32 = 0x0100;

pub struct InterpParams {
    pub n_inputs: u32,
    pub n_outputs: u32,
    pub n_samples: [u32; MAX_INPUT_DIMENSIONS],  // 各次元のグリッドサイズ
    pub domain: [u32; MAX_INPUT_DIMENSIONS],      // n_samples[i] - 1
    pub opta: [u32; MAX_INPUT_DIMENSIONS],        // ストライド（高速インデックス用）
    pub flags: u32,
}
```

#### 移植対象関数

| C版関数 | Rust版 | 備考 |
| ----------------------------- | ------------------------------------------------------ | ----------------------- |
| `_cmsComputeInterpParams` | `InterpParams::compute_uniform(n, n_in, n_out, flags)` | 全次元同一サイズ |
| `_cmsComputeInterpParamsEx` | `InterpParams::compute(samples, n_in, n_out, flags)` | 次元ごとにサイズ指定 |
| `_cmsFreeInterpParams` | (Drop) | 自動 |
| `Interpolation.Lerp16` | `params.eval_16(input, output, table)` | 16bit評価 |
| `Interpolation.LerpFloat` | `params.eval_float(input, output, table)` | float評価 |

#### 補間アルゴリズム（内部関数）

| 次元 | 16bit関数 | float関数 | 備考 |
| ---- | --------------------- | ------------------------- | -------------------------- |
| 1D | `lin_lerp_1d` | `lin_lerp_1d_float` | 単入力単出力 |
| 1D | `eval_1_input` | `eval_1_input_float` | 単入力多出力 |
| 2D | `bilinear_interp_16` | `bilinear_interp_float` | 双線形補間 |
| 3D | `trilinear_interp_16` | `trilinear_interp_float` | 三線形補間 |
| 3D | `tetrahedral_interp_16` | `tetrahedral_interp_float` | 坂本アルゴリズム（6四面体） |
| 4-15D | `eval_n_inputs_16` | `eval_n_inputs_float` | 再帰的次元分解 |

#### ヘルパー関数（pub(crate)、gamma.rs等で使用）

```rust
pub(crate) fn to_fixed_domain(a: i32) -> i32;
pub(crate) fn quick_saturate_word(d: f64) -> u16;
```

#### テスト

- 1D: 恒等テーブル全入力一致、gamma 3.0テーブル既知値検証
- 1D multi-output: 1入力3出力の補間
- 2D: 恒等LUT対角線検証
- 3D: tetrahedral/trilinear の恒等CLUT・既知値比較
- 3D: 16bitとfloatパスの整合性
- 4D+: CMYK用4次元恒等テーブル
- 境界: 入力0/0xFFFF、不正次元数エラー

#### プラグイン

`cmsInterpFnFactory`（補間アルゴリズム差し替え）はPhase 6で実装。

---

### 2. `src/curves/gamma.rs` — トーンカーブ

**C版対応**: `cmsgamma.c`（1,514行）

#### 主要構造体

```rust
/// カーブセグメント（パラメトリックまたはサンプル値）
#[derive(Clone, Debug)]
pub struct CurveSegment {
    pub x0: f32,
    pub x1: f32,
    pub curve_type: i32,         // >0: パラメトリック, 0: サンプル, <0: 逆関数
    pub params: [f64; 10],
    pub sampled_points: Vec<f32>, // curve_type==0の場合のみ
}

/// トーンカーブ: セグメント表現（float精度）+ 16bitテーブル（高速パス）
#[derive(Clone)]
pub struct ToneCurve {
    segments: Vec<CurveSegment>,
    seg_interp: Vec<Option<InterpParams>>,  // サンプルセグメント用
    table16: Vec<u16>,                       // 16bitルックアップテーブル
    interp_params: InterpParams,             // table16の補間パラメータ
}
```

**設計**: `ToneCurve`は`InterpParams`とテーブルデータを同一構造体に保持するが、`InterpParams`がテーブルへの参照を持たないため自己参照にならない。`eval_u16`メソッド内で`&self.table16`を`self.interp_params.eval_16()`に渡す。

#### セグメント評価: match dispatch

C版の関数ポインタ配列の代わりに、`match curve_type`で組み込み10型を分岐。プラグイン対応はPhase 6で追加。

```rust
fn eval_parametric(curve_type: i32, params: &[f64; 10], r: f64) -> f64 {
    match curve_type {
        1 => { /* Y = X^gamma */ }
        -1 => { /* Y = X^(1/gamma) */ }
        2 | -2 => { /* CIE 122-1966 */ }
        3 | -3 => { /* IEC 61966-3 */ }
        4 | -4 => { /* sRGB (IEC 61966-2.1) */ }
        5 | -5 => { /* 拡張sRGB */ }
        6 | -6 | 7 | -7 => { /* シグモイド */ }
        8 | -8 => { /* べき乗複合 */ }
        108 | -108 | 109 | -109 => { /* 拡張型 */ }
        _ => 0.0,
    }
}
```

#### 移植対象関数

**構築**

| C版関数 | Rust版 | 備考 |
| -------------------------------- | ------------------------------------------------ | ------------------------ |
| `cmsBuildGamma` | `ToneCurve::build_gamma(gamma)` | 単一ガンマ |
| `cmsBuildParametricToneCurve` | `ToneCurve::build_parametric(type, params)` | パラメトリック型1-8,108,109 |
| `cmsBuildTabulatedToneCurve16` | `ToneCurve::build_tabulated_16(values)` | 16bitテーブル |
| `cmsBuildTabulatedToneCurveFloat` | `ToneCurve::build_tabulated_float(values)` | floatテーブル |
| `cmsBuildSegmentedToneCurve` | `ToneCurve::build_segmented(segments)` | 複数セグメント |

**評価**

| C版関数 | Rust版 | 備考 |
| --------------------- | ----------------------- | ------------------------ |
| `cmsEvalToneCurveFloat` | `curve.eval_f32(v)` | セグメント評価 |
| `cmsEvalToneCurve16` | `curve.eval_u16(v)` | table16経由の補間 |

**ユーティリティ**

| C版関数 | Rust版 | 備考 |
| --------------------------- | --------------------------------- | -------------------------- |
| `cmsReverseToneCurve` | `curve.reverse()` | 逆カーブ生成 |
| `cmsReverseToneCurveEx` | `curve.reverse_with_samples(n)` | サンプル数指定 |
| `cmsJoinToneCurve` | `ToneCurve::join(x, y, n)` | 合成 Y(X(t)) |
| `cmsSmoothToneCurve` | `curve.smooth(lambda)` | Whittaker平滑化 |
| `cmsIsToneCurveLinear` | `curve.is_linear()` | |
| `cmsIsToneCurveMonotonic` | `curve.is_monotonic()` | |
| `cmsIsToneCurveDescending` | `curve.is_descending()` | |
| `cmsIsToneCurveMultisegment` | `curve.is_multisegment()` | |
| `cmsEstimateGamma` | `curve.estimate_gamma(precision)` | -1.0で失敗 |
| `cmsDupToneCurve` | `curve.clone()` (derive Clone) | 自動 |
| `cmsFreeToneCurve` | (Drop) | 自動 |

**アクセサ**

| C版関数 | Rust版 |
| ------------------------------------ | ---------------------------- |
| `cmsGetToneCurveParametricType` | `curve.parametric_type()` |
| `cmsGetToneCurveEstimatedTable` | `curve.table16()` |
| `cmsGetToneCurveEstimatedTableEntries` | `curve.table16_len()` |
| `cmsGetToneCurveSegment` | `curve.segment(n)` |

#### テスト

- パラメトリック型1（gamma 2.2）: 既知入力 [0.0, 0.25, 0.5, 0.75, 1.0] で `x^2.2` と比較
- パラメトリック型4（sRGB）: sRGBパラメータでの評価
- 全10型: 順方向→逆方向のround-trip
- 16bitテーブル: build → eval_u16 round-trip
- floatテーブル: build → eval_f32 round-trip
- reverse: gamma 2.2の逆カーブで `reversed(gamma(x)) ≈ x`
- join: gamma 2.2 + 逆 → 線形結果
- is_linear: 恒等カーブ=true、gamma 2.2=false
- is_monotonic: ガンマカーブは単調
- estimate_gamma: gamma 2.2構築→推定値≈2.2
- smooth: ノイズ付きテーブル→平滑化後も単調
- 境界: 空テーブルエラー、65530超エントリエラー

#### プラグイン

`_cmsRegisterParametricCurvesPlugin`（カスタムパラメトリック型登録）はPhase 6で実装。

---

### 3. `src/curves/wtpnt.rs` — 白色点・色順応

**C版対応**: `cmswtpnt.c`（353行）

#### 定数

```rust
/// Bradford色順応行列
pub const BRADFORD: Mat3 = Mat3([
    Vec3([ 0.8951,  0.2664, -0.1614]),
    Vec3([-0.7502,  1.7135,  0.0367]),
    Vec3([ 0.0389, -0.0685,  1.0296]),
]);
```

Robertson等温度データテーブル（31エントリ、`const`配列）。

#### 移植対象関数

| C版関数 | Rust版 | 備考 |
| ---------------------------------- | ---------------------------------------------------------------- | ----------------------- |
| `cmsWhitePointFromTemp` | `white_point_from_temp(temp_k: f64) -> Option<CieXyY>` | 4000-25000K |
| `cmsTempFromWhitePoint` | `temp_from_white_point(wp: &CieXyY) -> Option<f64>` | Robertson法 |
| `_cmsAdaptationMatrix` | `adaptation_matrix(cone, from, to) -> Option<Mat3>` | None=Bradford |
| `_cmsBuildRGB2XYZtransferMatrix` | `build_rgb_to_xyz_matrix(wp, primaries) -> Option<Mat3>` | RGB原色→XYZ行列 |
| `cmsAdaptToIlluminant` | `adapt_to_illuminant(src_wp, illuminant, value) -> Option<CieXyz>` | |
| `cmsD50_XYZ` | `d50_xyz() -> CieXyz` | |
| `cmsD50_xyY` | `d50_xyy() -> CieXyY` | |

#### 内部関数

- `compute_chromatic_adaptation(source_wp, dest_wp, chad) -> Option<Mat3>` — Bradford算法コア
- `adapt_matrix_to_d50(m, source_wp) -> Option<Mat3>` — D50への色順応適用

#### テスト

- 色温度round-trip: T=[4000,5000,5500,6000,6500,7000,8000,10000,15000,25000]で `|T-T'| < 0.5`
- 範囲外: T=3999, T=25001 → None
- Bradford: D65→D50順応行列を既知値と比較
- 恒等: 同一白色点→単位行列
- RGB2XYZ: sRGB原色+D65白色点→既知sRGB-to-XYZ行列
- adapt_to_illuminant: D50白色のD50→D65順応を既知値と比較

#### 依存

- `math/mtrx.rs`: `Vec3`, `Mat3`（inverse, eval, multiply）
- `math/pcs.rs`: `xyz_to_xyy`, `xyy_to_xyz`

---

### 4. `src/curves/cam02.rs` — CIECAM02

**C版対応**: `cmscam02.c`（490行）

#### 型定義

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum Surround {
    Average = 1,
    Dim = 2,
    Dark = 3,
    Cutsheet = 4,
}

pub const D_CALCULATE: f64 = -1.0;

pub struct ViewingConditions {
    pub white_point: CieXyz,
    pub yb: f64,          // 背景相対輝度
    pub la: f64,           // 順応光源輝度 (cd/m^2)
    pub surround: Surround,
    pub d_value: f64,      // 順応度（D_CALCULATEで自動計算）
}
```

#### 主要構造体

```rust
/// 事前計算済みCIECAM02モデル
pub struct CieCam02 {
    // 順応白色点の事前計算結果
    adopted_white: Cam02Color,
    // 環境パラメータ
    la: f64, yb: f64,
    f: f64, c: f64, nc: f64,
    n: f64, nbb: f64, ncb: f64,
    z: f64, fl: f64, d: f64,
}

// 内部色表現（CAM02パイプライン各段階の値を保持）
struct Cam02Color { /* xyz, rgb, rgb_c, rgb_p, rgb_pa, a, b, h, big_a, j, c */ }
```

#### 移植対象関数

| C版関数 | Rust版 | 備考 |
| -------------------- | ---------------------------------------- | ------------- |
| `cmsCIECAM02Init` | `CieCam02::new(vc: &ViewingConditions)` | 割り当て失敗なし |
| `cmsCIECAM02Done` | (Drop) | 自動 |
| `cmsCIECAM02Forward` | `model.forward(xyz: &CieXyz) -> JCh` | XYZ→JCh |
| `cmsCIECAM02Reverse` | `model.reverse(jch: &JCh) -> CieXyz` | JCh→XYZ |

#### 変換パイプライン

**順方向**: XYZ → CAT02行列 → 色順応 → HPE行列 → 非線形圧縮 → 相関量計算 → JCh
**逆方向**: JCh → 逆相関量 → 逆非線形 → HPE逆行列 → 逆色順応 → CAT02逆行列 → XYZ

固定行列（CAT02, CAT02逆, HPE）は`const`配列として定義。

#### テスト

- Forward-reverse round-trip: 複数XYZ値（標準観察条件: D50, La=200, Yb=20, Average）
- D50白色点: forward → J≈100, C≈0
- 各Surround条件（Average/Dim/Dark/Cutsheet）: 同一XYZで異なるJ値
- D_CALCULATE: La から D を自動計算
- 境界: 極暗色（ゼロ近傍XYZ）でNaN/panic回避

---

## lib.rs への追加

```rust
pub mod curves;  // 追加
```

## src/curves/mod.rs

```rust
pub mod intrp;
pub mod gamma;
pub mod wtpnt;
pub mod cam02;
```

---

## TDDコミット順序

| # | 種別 | コミットメッセージ | 内容 |
| -- | -------- | ----------------------------------------------------------- | ------------------------------------------------- |
| 1 | RED | `test(intrp): add 1D interpolation tests` | 1D 16bit/float テスト（`#[ignore]`） |
| 2 | GREEN | `feat(intrp): implement InterpParams and 1D interpolation` | InterpParams, 1D補間実装 |
| 3 | RED | `test(intrp): add 3D interpolation tests` | 2D/3D tetrahedral/trilinear テスト（`#[ignore]`） |
| 4 | GREEN | `feat(intrp): implement 2D bilinear and 3D interpolation` | 2D/3D補間実装 |
| 5 | RED | `test(intrp): add multi-dimensional interpolation tests` | 4D+ テスト（`#[ignore]`） |
| 6 | GREEN | `feat(intrp): implement 4D-15D recursive interpolation` | N次元再帰補間 |
| 7 | RED | `test(gamma): add parametric tone curve tests` | 組み込み型1-8,108,109 テスト（`#[ignore]`） |
| 8 | GREEN | `feat(gamma): implement parametric tone curves` | CurveSegment, ToneCurve, eval_parametric |
| 9 | RED | `test(gamma): add tabulated and segmented tone curve tests` | テーブル/セグメントカーブ テスト（`#[ignore]`） |
| 10 | GREEN | `feat(gamma): implement tabulated and segmented tone curves` | build_tabulated, build_segmented |
| 11 | RED | `test(gamma): add tone curve utilities tests` | reverse/join/smooth等 テスト（`#[ignore]`） |
| 12 | GREEN | `feat(gamma): implement tone curve utilities` | reverse, join, smooth, 検査関数 |
| 13 | RED | `test(wtpnt): add white point and chromatic adaptation tests` | 色温度/Bradford テスト（`#[ignore]`） |
| 14 | GREEN | `feat(wtpnt): implement white point and chromatic adaptation` | 全関数実装 |
| 15 | RED | `test(cam02): add CIECAM02 forward and reverse tests` | forward/reverse round-trip テスト（`#[ignore]`） |
| 16 | GREEN | `feat(cam02): implement CIECAM02 appearance model` | 全モデル実装 |
| 17 | docs | `docs(plans): update Phase 2 status to IMPLEMENTED` | ステータス更新 |

---

## 技術的リスクと対策

| リスク | 対策 |
| -------------------------------- | ------------------------------------------------------------------- |
| InterpParams + ToneCurve自己参照 | InterpParamsはテーブル非参照。eval時に`&self.table16`を引数で渡す |
| 16bit固定小数点のオーバーフロー | C版と同一の`i32`演算+明示的キャスト。C版テスト値で検証 |
| N次元再帰補間（4D-15D） | 再帰関数で次元を1つずつ剥がし、3Dでtetrahedralをbase case |
| Whittaker平滑化の1-based配列 | C版に合わせて`n+1`要素確保しindex 0を無視（移植正確性を優先） |
| 浮動小数点精度のC版との差異 | float操作は相対誤差1e-5、固定小数点は完全一致 |

## Phase 6へ先送りする機能

- 補間プラグイン（`cmsInterpFnFactory`）
- パラメトリックカーブプラグイン（`_cmsRegisterParametricCurvesPlugin`）
- Context threading in InterpParams

## 検証方法

```bash
cargo test                                      # 全テスト通過
cargo clippy --all-targets -- -D warnings       # 警告ゼロ
cargo fmt -- --check                            # フォーマット準拠
```

完了基準（全体戦略計画書より）: ガンマ2.2カーブ評価、3D LUT補間、D50↔D65色順応がテスト通過
