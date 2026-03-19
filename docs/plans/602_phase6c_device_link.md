# Phase 6c: Transform::to_device_link() デバイスリンクプロファイル変換

**Status**: IMPLEMENTED
**C版ファイル**: `cmsvirt.c`（`cmsTransform2DeviceLink`）
**Rust見積**: ~200行（impl）+ ~100行（tests）
**ブランチ**: `feat/phase6c-device-link`

## Context

Phase 6b でプロファイル生成関数を完了。`cmsvirt.c` の最後の主要関数
`cmsTransform2DeviceLink` を移植し、既存のTransformをデバイスリンクプロファイルに
変換する機能を追加する。

## 変更対象ファイル

| ファイル                 | 操作                                      |
| ------------------------ | ----------------------------------------- |
| `src/transform/xform.rs` | `to_device_link()`、AllowedLUT テーブル等 |
| `src/profile/virt.rs`    | `set_text_tags_fallible()` 公開           |

## 実装する関数

### xform.rs 変更

| 変更                           | 内容                                        |
| ------------------------------ | ------------------------------------------- |
| `entry_color_space` フィールド | 入力プロファイルの色空間（Transformに保持） |
| `exit_color_space` フィールド  | 出力プロファイルの色空間（Transformに保持） |
| `rendering_intent` フィールド  | レンダリングインテント（Transformに保持）   |
| `pipeline()` アクセサ          | 内部パイプラインの参照取得                  |
| `to_device_link()` メソッド    | Transform → デバイスリンクProfile変換       |

### virt.rs 追加

| 関数                 | C版                 | 内容                              |
| -------------------- | ------------------- | --------------------------------- |
| `find_combination()` | `FindCombination`   | パイプラインの許可LUT型マッチング |
| `fix_color_spaces()` | `FixColorSpaces`    | デバイスクラス・色空間の自動決定  |
| AllowedLUTTypes定数  | `AllowedLUTTypes[]` | V2/V4のステージ組合せ許可テーブル |

## 処理フロー

### to_device_link()

```text
1. パイプラインを clone
2. Lab V2/V4 エンコーディング補正（version < 4.0 時）:
   - entry=Lab → LabV2toV4 curves を先頭に挿入
   - exit=Lab → LabV4toV2 を末尾に挿入
3. プロファイル作成、FixColorSpaces でヘッダ設定
4. FindCombination で許可LUT型を検索:
   a) 直接マッチ → 使用
   b) マッチなし → optimize_pipeline 後に再検索
   c) まだマッチなし → FORCE_CLUT + identity curves 追加 → 再検索
5. AToB0 または BToA0 タグに書き込み
6. MediaWhitePoint タグ書き込み（D50）
```

### AllowedLUTTypes テーブル

```text
V2 (Lut16Type):
  [Matrix, CurveSet, CLUT, CurveSet]
  [CurveSet, CLUT, CurveSet]
  [CurveSet, CLUT]

V4 AToB (LutAtoBType):
  [CurveSet]
  [CurveSet, Matrix, CurveSet]
  [CurveSet, CLUT, CurveSet]
  [CurveSet, CLUT, CurveSet, Matrix, CurveSet]

V4 BToA (LutBtoAType):
  [CurveSet]
  [CurveSet, Matrix, CurveSet]
  [CurveSet, CLUT, CurveSet]
  [Matrix, CurveSet, CLUT, CurveSet, CurveSet]
```

## Deferred

- Named color device link（NamedColorElem ステージ検出 → 専用処理）
- InputColorant / OutputColorant タグ書き込み
- ProfileSequenceDescription タグ書き込み
- FLAGS_8BITS_DEVICELINK（8bit保存モード）
- FLAGS_GUESSDEVICECLASS（デバイスクラス自動推定）

## コミット構成（TDD）

### Commit 1: 計画書

### Commit 2 (RED): テスト

- sRGB→sRGB Transform → device link: ヘッダ・AToB0タグ検証
- sRGB→Lab Transform → device link: AToB0タグ・色空間検証
- device link round-trip: 元の変換と同等の出力

### Commit 3 (GREEN): 実装

## 検証方法

```bash
cargo test device_link
cargo test xform
cargo test virt
cargo test
cargo clippy --all-targets -- -D warnings
cargo fmt -- --check
```
