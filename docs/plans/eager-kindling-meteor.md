# Phase 14: Remaining API Gaps

Status: IMPLEMENTED

## Context

Phase 13 完了後のカバレッジレポートに基づき、残りの未実装関数を精査した結果、
プラグイン/グローバル状態/メモリ管理系（Rustでは不要）を除外した上で、
ライブラリユーザーにとって実用的な関数を優先度順に実装する。

## 対象

| # | 機能                     | C関数                           | Rustファイル                                     | 規模   |
| - | ------------------------ | ------------------------------- | ------------------------------------------------ | ------ |
| A | インテント列挙           | `cmsGetSupportedIntents`        | `src/transform/cnvrt.rs`                         | ~30行  |
| B | PS統合ディスパッチャ     | `cmsGetPostScriptColorResource` | `src/ext/ps2.rs`                                 | ~10行  |
| C | PCSフォーマッタ          | `cmsFormatterForPCSOfProfile`   | `src/pipeline/pack.rs`                           | ~15行  |
| D | CGATS数値API             | `cmsIT8SetPropertyDbl` 等       | `src/ext/cgats.rs`                               | ~40行  |
| E | Named Color Stage        | `_cmsStageAllocNamedColor`      | `src/pipeline/lut.rs`                            | ~40行  |
| F | Black-Preserving K-Only  | `BlackPreservingKOnlyIntents`   | `src/transform/cnvrt.rs`, `src/transform/gmt.rs` | ~150行 |
| G | Black-Preserving K-Plane | `BlackPreservingKPlaneIntents`  | `src/transform/cnvrt.rs`, `src/transform/gmt.rs` | ~200行 |

## 除外（Rustでは不要 / 設計差異）

- プラグイン登録系: `_cmsRegister*Plugin`, `_cmsAllocPluginChunk`
- グローバル状態: `cmsSetAlarmCodes`, `cmsSetAdaptationState` (THRなし版)
- Worker/並列化: `_cmsGetTransformWorker*`
- メモリ管理: `_cmsMalloc`, SubAlloc等
- Stream I/O: `cmsOpenIOhandlerFromStream` (file/memで充分)
- Wide文字列: `cmsMLUsetWide` (UTF-8 APIで代替)
- MPE/MultiProcessElement: 高度すぎるため後回し

## PR構成

### PR 1: feat/phase14a-convenience-apis（A, B, C, D）

小さな便利関数の一括実装。

#### A. `get_supported_intents()`

`src/transform/cnvrt.rs` に関数追加。

```rust
pub fn get_supported_intents() -> &'static [(u32, &'static str)] {
    &[
        (0, "Perceptual"),
        (1, "Relative Colorimetric"),
        (2, "Saturation"),
        (3, "Absolute Colorimetric"),
    ]
}
```

- C版参照: `cmscnvrt.c:945-982`
- C版はプラグインチェーンを走査するが、Rustでは固定セット

#### B. `get_postscript_color_resource()`

`src/ext/ps2.rs` に関数追加。

```rust
pub fn get_postscript_color_resource(
    resource_type: PostScriptResourceType,
    profile: &mut Profile,
    intent: u32,
    flags: u32,
) -> Result<Vec<u8>, CmsError>
```

- `PostScriptResourceType::Csa` → `get_postscript_csa()` に委譲
- `PostScriptResourceType::Crd` → `get_postscript_crd()` に委譲
- C版参照: `cmsps2.c:831-848`

#### C. `formatter_for_pcs_of_profile()`

`src/pipeline/pack.rs` に関数追加。

- `profile.header.pcs` が `LabData` → `TYPE_Lab_DBL` / `TYPE_Lab_16` 相当のフォーマッタ
- `profile.header.pcs` が `XyzData` → `TYPE_XYZ_DBL` / `TYPE_XYZ_16` 相当
- `is_float` フラグで float/16bit を切り替え
- C版参照: `cmspack.c:3428-3445`

#### D. CGATS数値API

`src/ext/cgats.rs` に `impl It8` メソッド追加。

- `set_property_f64(key, value)`: `format!("{}", value)` → `set_property()`
- `set_data_row_col_f64(row, col, value)`: `format!("{}", value)` → `set_data_row_col()`
- `set_data_f64(patch, sample, value)`: `format!("{}", value)` → `set_data()`
- `get_patch_name(row)`: `data_row_col(row, 0)` で SAMPLE_ID 列を返す
- `define_dbl_format(fmt)`: フォーマット文字列を保存（save_to_string で使用）

既存メソッド `set_property()`, `set_data_row_col()`, `set_data()`, `data_row_col()` を再利用。

#### コミット順序

1. `test(cnvrt,ps2,pack,cgats): add convenience API tests` [RED]
2. `feat(cnvrt): implement get_supported_intents` [GREEN]
3. `feat(ps2): implement get_postscript_color_resource` [GREEN]
4. `feat(pack): implement formatter_for_pcs_of_profile` [GREEN]
5. `feat(cgats): implement numeric setter/getter APIs` [GREEN]

---

### PR 2: feat/phase14b-named-color-stage（E）

Named Color パイプラインステージ。

#### E. `Stage::new_named_color()`

`src/pipeline/lut.rs` に追加。

コンストラクタ:

```rust
pub fn new_named_color(list: NamedColorList, use_pcs: bool) -> Self
```

- `input_channels = 1`（インデックス入力）
- `output_channels = if use_pcs { 3 } else { list.colorant_count() }`
- `stage_type = StageSignature::NamedColorElem`
- `data = StageData::NamedColor(list)`

eval ディスパッチ（`Stage::eval()` の match に追加）:

```rust
StageSignature::NamedColorElem => {
    if let StageData::NamedColor(ref list) = self.data {
        let index = (input[0] * 65535.0 + 0.5) as usize;
        let index = index.min(list.len().saturating_sub(1));
        if let Some(color) = list.info(index) {
            // use_pcs: output PCS values, else: device colorant values
            // PCS values are in color.pcs[0..3]
        }
    }
}
```

- C版参照: `cmsnamed.c:899-945`
- 既存: `NamedColorList` (`src/pipeline/named.rs`), `StageData::NamedColor` (バリアント定義済)

#### コミット順序

1. `test(lut): add named color stage tests` [RED]
2. `feat(lut): implement Stage::new_named_color and eval` [GREEN]

---

### PR 3: feat/phase14c-black-preserving（F, G）

Black-Preserving インテント。CMYK ワークフローで K チャンネルを保持する。

#### 依存関数

`src/transform/gmt.rs` に追加:

- `chain_to_lab()`: プロファイル配列 → Lab パイプライン構築
  - プロファイルを save/load でクローンし、Lab V4 プロファイルを末尾に追加
  - `Transform::new_multiprofile()` で変換生成
  - C版参照: `cmsgmt.c:288-337`

- `build_k_tone_curve()`: K チャンネルのトーンカーブ構築
  - CMYK→CMYK 変換を作成し、K=0..255, CMY=0 で評価
  - 結果から `ToneCurve::build_tabulated_16()` を生成
  - C版参照: `cmsgmt.c:340-425`

#### F. `black_preserving_k_only_intents()`

`src/transform/cnvrt.rs` に追加。

1. CMYK→Lab 変換構築（`chain_to_lab()`）
2. Lab→CMYK 変換構築（通常の `default_icc_intents()`）
3. パイプライン: 入力 CMYK → Lab → CMYK だが K は入力から直接コピー
4. K チャンネルの差し替えは CLUT サンプリングで実現

C版参照: `cmscnvrt.c:675-756`

#### G. `black_preserving_k_plane_intents()`

`src/transform/cnvrt.rs` に追加。

Fの拡張版。K トーンカーブを使い、CMY チャンネルも K との相互作用を補正。

C版参照: `cmscnvrt.c:804-925`

#### インテントディスパッチ更新

`link_profiles()` の match にインテント 10, 11 を追加:

- 10 → `black_preserving_k_only_intents()`
- 11 → `black_preserving_k_plane_intents()`

`get_supported_intents()` にも追加。

#### コミット順序

1. `test(gmt): add chain_to_lab tests` [RED]
2. `feat(gmt): implement chain_to_lab` [GREEN]
3. `test(gmt): add build_k_tone_curve tests` [RED]
4. `feat(gmt): implement build_k_tone_curve` [GREEN]
5. `test(cnvrt): add black-preserving K-only intent tests` [RED]
6. `feat(cnvrt): implement black_preserving_k_only_intents` [GREEN]
7. `test(cnvrt): add black-preserving K-plane intent tests` [RED]
8. `feat(cnvrt): implement black_preserving_k_plane_intents` [GREEN]

## 依存関係

```text
PR 1 (14a) ── 独立 ──→ 最初にマージ
PR 2 (14b) ── 独立 ──→ 任意の順序
PR 3 (14c) ── PR 1後 → 最後にマージ（get_supported_intents 更新のため）
```

## 検証

各PRで以下を確認:

```bash
cargo test
cargo clippy --all-targets -- -D warnings
cargo fmt -- --check
```

Phase 14a のテスト例:

- `get_supported_intents()` が4エントリを返す
- PostScript リソース取得でCSA/CRD選択が正しい
- sRGB の PCS フォーマッタが Lab 系を返す
- IT8 の数値プロパティ設定→読み取りラウンドトリップ

Phase 14b のテスト例:

- NamedColorList から stage 構築、インデックス入力で PCS 出力
- 範囲外インデックスのクランプ

Phase 14c のテスト例:

- CMYK→CMYK 変換で K チャンネルが保持される
- K-only と K-plane で結果が異なる（CMY 補正の有無）
