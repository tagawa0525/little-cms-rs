# Phase 5b: xform.rs Transform構造体

**Status**: IMPLEMENTED
**C版ファイル**: `cmsxform.c`（1,475行）
**Rust見積**: ~350行（impl）+ ~200行（tests）
**ブランチ**: `feat/phase5b-xform`

## Context

Phase 5a（cnvrt.rs: パイプライン構築）と Phase 3c（pack.rs: ピクセルフォーマッタ）が完了。
これらを結合して end-to-end の色変換を実行する `Transform` 構造体を実装する。

## 変更対象ファイル

| ファイル                 | 操作                                      |
| ------------------------ | ----------------------------------------- |
| `src/transform/xform.rs` | 新規作成                                  |
| `src/transform/mod.rs`   | `pub mod xform;` 追加                     |
| `src/types.rs`           | `ColorSpaceSignature::to_pixel_type()` 等 |

## 実装する型・関数

### types.rs 追加

| 関数                                             | C版                  | 内容                     |
| ------------------------------------------------ | -------------------- | ------------------------ |
| `ColorSpaceSignature::to_pixel_type(&self)->u32` | `_cmsLCMScolorSpace` | ICC署名→PT_*インデックス |
| `ColorSpaceSignature::from_pixel_type(u32)->..`  | `_cmsICCcolorSpace`  | PT_*インデックス→ICC署名 |

### xform.rs

| 型/関数                         | C版                              | 内容                    |
| ------------------------------- | -------------------------------- | ----------------------- |
| `Transform` struct              | `_cmsTRANSFORM`                  | 変換ハンドル            |
| `Transform::new()`              | `cmsCreateTransform`             | 2プロファイル変換の作成 |
| `Transform::new_multiprofile()` | `cmsCreateMultiprofileTransform` | Nプロファイル変換の作成 |
| `transform.do_transform()`      | `cmsDoTransform`                 | ピクセル列の変換実行    |
| `transform.input_format()`      | `cmsGetTransformInputFormat`     | 入力フォーマット取得    |
| `transform.output_format()`     | `cmsGetTransformOutputFormat`    | 出力フォーマット取得    |

### Transform struct

```rust
pub struct Transform {
    pipeline: Pipeline,
    input_format: PixelFormat,
    output_format: PixelFormat,
    from_input: FormatterIn,
    to_output: FormatterOut,
    entry_color_space: ColorSpaceSignature,
    exit_color_space: ColorSpaceSignature,
    rendering_intent: u32,
    flags: u32,
}
```

### フラグ定数

```rust
pub const FLAGS_NOCACHE: u32 = 0x0040;
pub const FLAGS_NOOPTIMIZE: u32 = 0x0100;
pub const FLAGS_NULLTRANSFORM: u32 = 0x0200;
pub const FLAGS_GAMUTCHECK: u32 = 0x1000;
pub const FLAGS_SOFTPROOFING: u32 = 0x4000;
pub const FLAGS_BLACKPOINTCOMPENSATION: u32 = 0x2000;
```

## 処理フロー

### Transform::new()

```text
1. entry_cs = profiles[0].header.color_space
2. exit_cs  = profiles[1].header.pcs or color_space (depending on class)
3. input_format の colorspace が entry_cs と互換か検証
4. output_format の colorspace が exit_cs と互換か検証
5. pipeline = link_profiles(profiles, intents, bpc, adaptation)
6. pipeline の in/out channels が format と一致するか検証
7. Formatter lookup (16-bit or float)
8. Transform 構築
```

### do_transform()

```text
for each pixel:
  1. from_input: buf → work array (u16[] or f32[])
  2. pipeline.eval_16/eval_float: work_in → work_out
  3. to_output: work array → output buf
```

## Scope

本PRの実装範囲:

- Transform struct + creation + execution
- 16-bit / float 両パス
- ColorSpaceSignature ↔ PT_* 変換
- フラグ定数

今後のPhaseで追加:

- 1-pixel cache（パフォーマンス最適化）
- Gamut check（cmsgmt.rs Phase）
- Pipeline optimization（opt.rs Phase）
- Null transform
- ChangeBuffersFormat
- Plugin transform（plugin.rs Phase）

## コミット構成（TDD）

### Commit 1: 計画書

### Commit 2 (RED): テスト

- `ColorSpaceSignature::to_pixel_type` / `from_pixel_type`
- `Transform::new` 2-profile RGB→RGB (identity-ish)
- `do_transform` RGB_8 → RGB_8 round-trip
- `do_transform` RGB_8 → Lab (via float)
- `input_format()` / `output_format()` クエリ
- エラーケース: フォーマット不一致

### Commit 3 (GREEN): 実装

## 検証方法

```bash
cargo test xform
cargo test
cargo clippy --all-targets -- -D warnings
cargo fmt -- --check
```
