# Phase 10: Gamut Check Pipeline + Proofing Transform

**Status**: IMPLEMENTED
**C版ファイル**: `cmsgmt.c`（deferred部分）+ `cmsxform.c`（proofing/gamut統合）
**Rust見積**: ~250行（impl）+ ~150行（tests）
**ブランチ**: `feat/phase10-gamut-check`

## Context

Phase 5e で gmt.rs の基本機能（detect_tac, detect_rgb_profile_gamma, desaturate_lab, build_k_tone_curve）を実装した。
`create_gamut_check_pipeline()` と `GamutSampler` は xform.rs の FLAGS_GAMUTCHECK 統合が前提のため deferred としていた。
本フェーズでは gamut check パイプライン構築と proofing transform を実装し、ソフトプルーフ＋色域外警告を完成させる。

## スコープ

### 実装する機能

1. **`create_gamut_check_pipeline()`** — 入力色→色域内外判定（1ch CLUT）パイプライン構築
2. **`GamutSampler`** — Lab ラウンドトリップで色域内外を判定する CLUT サンプリングコールバック
3. **`Transform::new_proofing()`** — proofing transform 作成 API（FLAGS_SOFTPROOFING / FLAGS_GAMUTCHECK）
4. **Gamut check 付き変換実行** — `do_transform` で gamut check パイプラインを評価し、色域外に alarm codes を出力
5. **Alarm codes API** — `Context` のデフォルト値設定、`Transform` への alarm codes 格納

### Deferred

- Context ベースの per-thread alarm codes（グローバル Context 未実装のため、Transform にインライン格納）
- `cmsCreateExtendedTransform`（多プロファイル + 個別 intent/BPC/adaptation）

## 変更対象ファイル

| ファイル                 | 操作                                    |
| ------------------------ | --------------------------------------- |
| `src/transform/gmt.rs`   | `create_gamut_check_pipeline` + sampler |
| `src/transform/xform.rs` | proofing transform + gamut check 評価   |
| `src/context.rs`         | alarm codes デフォルト値                |

## 実装する関数

### gmt.rs

| 関数                            | C版                            | 内容                                     |
| ------------------------------- | ------------------------------ | ---------------------------------------- |
| `create_gamut_check_pipeline()` | `_cmsCreateGamutCheckPipeline` | 色域判定 CLUT パイプライン構築           |
| `gamut_sampler()`               | `GamutSampler`                 | Lab ラウンドトリップ dE 判定コールバック |

### xform.rs

| 関数                        | C版                               | 内容                                   |
| --------------------------- | --------------------------------- | -------------------------------------- |
| `Transform::new_proofing()` | `cmsCreateProofingTransformTHR`   | proofing transform 作成                |
| gamut check 付き評価        | `TransformOnePixelWithGamutCheck` | gamut check 分岐を do_transform に統合 |

### context.rs

| 変更                   | C版                   | 内容                               |
| ---------------------- | --------------------- | ---------------------------------- |
| alarm codes デフォルト | `_cmsAlarmCodesChunk` | `[0x7F00, 0x7F00, 0x7F00, 0, ...]` |

## 処理フロー

### create_gamut_check_pipeline()

```text
1. プロファイルチェーン（入力→PCS位置まで）から Lab 変換を構築
   - hInput: profiles[0..=nGamutPCSposition] の16bit→Lab float 変換
2. gamut プロファイルから forward/reverse 変換を構築
   - hForward: Lab → gamut profile colorants (RelativeColorimetric)
   - hReverse: gamut colorants → Lab (RelativeColorimetric)
3. gamut profile の CLUT 有無で threshold を決定
   - matrix-shaper → threshold = 1.0
   - CLUT ベース → threshold = 5.0 (ERR_THRESHOLD)
4. 1ch CLUT パイプライン生成（gridpoints は reasonable_gridpoints で決定）
5. GamutSampler で CLUT をサンプリング
6. パイプラインを返す
```

### GamutSampler

```text
入力: Lab 色（16bit → float 変換済み）
1. Lab → colorants (hForward)
2. colorants → Lab' (hReverse)
3. dE1 = distance(Lab, Lab')
4. colorants' → Lab'' (hForward → hReverse)
5. dE2 = distance(Lab', Lab'')
6. 判定:
   - dE1 < threshold && dE2 < threshold → 0（色域内）
   - dE1 < threshold && dE2 > threshold → 0（不定、色域内扱い）
   - dE1 > threshold && dE2 < threshold → dE1 - threshold（色域外）
   - dE1 > threshold && dE2 > threshold → max(dE1, dE2) ベース（知覚マッピング）
7. 出力値を [0, 0xFFFE] にクランプ
```

### Transform::new_proofing()

```text
1. 入力: input_profile, output_profile, proofing_profile, intent, proofing_intent, flags
2. プロファイルチェーン構築:
   - profiles = [input, proofing, proofing, output]
   - intents = [intent, proofing_intent, INTENT_RELATIVE_COLORIMETRIC, intent]
   - BPC = [bpc, bpc, false, false]
3. FLAGS_GAMUTCHECK → create_gamut_check_pipeline(profiles, bpc, intents, adaptation, 1, proofing_profile)
4. FLAGS_SOFTPROOFING がなければ通常の2プロファイル変換にフォールバック
5. パイプライン構築 → 最適化 → Transform 生成
```

### Gamut check 付き変換実行

```text
16bit パス:
  1. pixel を unroll → w_in
  2. gamut_check パイプラインで w_in を評価 → out_of_gamut (1ch)
  3. out_of_gamut > 0 → alarm_codes を w_out にコピー
  4. out_of_gamut == 0 → 通常パイプラインで w_in → w_out
  5. w_out を pack

float パス:
  1. pixel を unroll → w_in
  2. gamut_check パイプラインで w_in を評価 → out_of_gamut (1ch float)
  3. out_of_gamut > 0.0 → alarm_codes / 65535.0 を w_out にコピー
  4. out_of_gamut == 0.0 → 通常パイプラインで w_in → w_out
  5. w_out を pack
```

## コミット構成（TDD）

### Commit 1: 計画書

### Commit 2 (RED): テスト

- `gamut_sampler_in_gamut`: sRGB 中間グレーは sRGB gamut 内 → 0
- `gamut_sampler_out_of_gamut`: sRGB の gamut 外 Lab 値 → >0
- `create_gamut_check_pipeline_srgb`: sRGB gamut check パイプライン構築・評価
- `proofing_transform_basic`: proofing transform 作成と基本変換
- `proofing_transform_gamut_check_alarm`: FLAGS_GAMUTCHECK で色域外に alarm codes 出力
- `alarm_codes_default`: Context の alarm codes デフォルト値検証

### Commit 3 (GREEN): 実装

## 検証方法

```bash
cargo test gmt
cargo test xform
cargo test
cargo clippy --all-targets -- -D warnings
cargo fmt -- --check
```
