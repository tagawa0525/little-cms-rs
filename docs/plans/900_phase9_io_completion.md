# Phase 9: io.rs プロファイルI/O完成（Phase 4d 残り）

**Status**: IMPLEMENTED
**C版ファイル**: `cmsio1.c`（未移植部分: ~150行相当）
**Rust見積**: ~150行（impl）+ ~100行（tests）
**ブランチ**: `feat/phase9-io-completion`

## Context

Phase 4d で基本的なパイプライン構築（`read_input_lut`, `read_output_lut`, matrix-shaper ビルダー）を実装した。
本フェーズでは残りの関数と Lab V2↔V4 自動挿入を追加し、プロファイル I/O を完成させる。

## スコープ

### 実装する機能

1. **`read_devicelink_lut()`** — DeviceLink/Abstract プロファイルのパイプライン読み取り
2. **`is_clut()`** — CLUT ベースプロファイル判定
3. **`is_intent_supported()`** — レンダリングインテントサポート判定
4. **Lab V2↔V4 自動挿入** — `read_input_lut` / `read_output_lut` / `read_devicelink_lut` で Lut16Type の Lab PCS 処理
5. **`tag_true_type()`** — タグの実際の型シグネチャ取得（Lut16 判定用）
6. **`Pipeline::change_interp_to_trilinear()`** — Lab PCS 時の CLUT 補間モード変更

### Deferred

- Float タグ（DToB/BToD）サポート — 大規模変更、別フェーズ
- Named Color プロファイルの read_input_lut/read_devicelink_lut 対応
- `cmsGetSupportedIntents` / Intent plugin 登録

## 変更対象ファイル

| ファイル              | 操作                           |
| --------------------- | ------------------------------ |
| `src/profile/io.rs`   | 修正                           |
| `src/pipeline/lut.rs` | 修正（trilinear 変更ヘルパー） |

## 実装する関数

### io.rs

| 関数                    | C版                     | 内容                                     |
| ----------------------- | ----------------------- | ---------------------------------------- |
| `tag_true_type()`       | `_cmsGetTagTrueType`    | タグの実際の型シグネチャを返す           |
| `read_devicelink_lut()` | `_cmsReadDevicelinkLUT` | DeviceLink/Abstract パイプライン読み取り |
| `is_clut()`             | `cmsIsCLUT`             | CLUT ベース判定                          |
| `is_intent_supported()` | `cmsIsIntentSupported`  | インテントサポート判定                   |

### lut.rs

| 関数                                     | C版                              | 内容                                   |
| ---------------------------------------- | -------------------------------- | -------------------------------------- |
| `Pipeline::change_interp_to_trilinear()` | `ChangeInterpolationToTrilinear` | CLUT ステージの補間を trilinear に変更 |

## 処理フロー

### tag_true_type()

```text
1. 指定タグの raw bytes を取得（先頭 4 bytes のみ必要）
2. 先頭 4 bytes を TagTypeSignature としてパース
3. TagTypeSignature を返す
```

### read_devicelink_lut()

```text
1. intent > 3 → エラー
2. tag16 = DEVICE2PCS16[intent]
   タグが無ければ Perceptual (DEVICE2PCS16[0]) にフォールバック
   両方無ければエラー
3. パイプラインを read_tag() で取得
4. PCS が Lab → change_interp_to_trilinear()
5. OriginalType = tag_true_type()
   OriginalType != Lut16 → そのまま返す
6. Lut16Type の Lab V2↔V4 処理:
   - color_space が Lab → 先頭に LabV4ToV2 挿入
   - PCS が Lab → 末尾に LabV2ToV4 挿入
7. パイプラインを返す
```

### Lab V2↔V4 自動挿入（read_input_lut 修正）

```text
既存の read_input_lut に追加:
1. LUT タグ読み取り成功後:
   a. PCS が Lab → change_interp_to_trilinear()
   b. OriginalType = tag_true_type()
   c. OriginalType != Lut16 → そのまま返す
   d. color_space が Lab → 先頭に LabV4ToV2 挿入
   e. 末尾に LabV2ToV4 挿入（V2→V4 PCS 正規化）
```

### Lab V2↔V4 自動挿入（read_output_lut 修正）

```text
既存の read_output_lut に追加:
1. LUT タグ読み取り成功後:
   a. PCS が Lab → change_interp_to_trilinear()
   b. OriginalType = tag_true_type()
   c. OriginalType != Lut16 → そのまま返す
   d. 先頭に LabV4ToV2 挿入（V4→V2 PCS 変換）
   e. color_space が Lab → 末尾に LabV2ToV4 挿入
```

### is_clut()

```text
1. DeviceLink → ヘッダの rendering intent と一致するか返す
2. direction に応じてタグテーブル選択:
   - Input → DEVICE2PCS16
   - Output → PCS2DEVICE16
   - Proof → is_intent_supported(intent, Input) && is_intent_supported(RelCol, Output)
3. intent > 3 → false
4. タグテーブル[intent] が存在するか返す
```

### is_intent_supported()

```text
1. is_clut() → true
2. is_matrix_shaper() → true
3. false
```

### Pipeline::change_interp_to_trilinear()

```text
1. 全ステージを走査
2. CLutElem ステージの InterpParams に TRILINEAR フラグを設定
3. fast_eval16 を無効化（プリコンパイル済み補間パラメータが陳腐化するため）
```

## コミット構成（TDD）

### Commit 1: 計画書

### Commit 2 (RED): テスト

- `tag_true_type`: sRGB プロファイルの AToB0 タグ型を検証
- `read_devicelink_lut`: DeviceLink プロファイルからパイプライン読み取り
- `is_clut`: CLUT ベース / matrix-shaper の判定
- `is_intent_supported`: CLUT/matrix-shaper のフォールバック判定
- Lab V2↔V4: Lut16 + Lab PCS のパイプラインにステージ挿入検証

### Commit 3 (GREEN): 実装

## 検証方法

```bash
cargo test io
cargo test
cargo clippy --all-targets -- -D warnings
cargo fmt -- --check
```
