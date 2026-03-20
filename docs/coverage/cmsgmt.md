# cmsgmt.c カバレッジ

- **C ソース**: `reference/Little-CMS/src/cmsgmt.c`
- **Rust ファイル**: `src/transform/gmt.rs`
- **概要**: ガマットマッピング・TAC検出・ガマットチェック

## 公開API

| C 関数                     | Rust 対応                    | 状態   |
| -------------------------- | ---------------------------- | ------ |
| `cmsDetectTAC`             | `detect_tac()`               | 実装済 |
| `cmsDesaturateLab`         | `desaturate_lab()`           | 実装済 |
| `cmsDetectRGBProfileGamma` | `detect_rgb_profile_gamma()` | 実装済 |

## 内部関数

| C 関数                         | Rust 対応                       | 状態   |
| ------------------------------ | ------------------------------- | ------ |
| `_cmsChain2Lab`                | —                               | 未実装 |
| `_cmsBuildKToneCurve`          | —                               | 未実装 |
| `_cmsCreateGamutCheckPipeline` | `create_gamut_check_pipeline()` | 実装済 |

## 備考

- `_cmsChain2Lab`: プロファイルからLabへの変換パイプライン構築。Black-Preservingインテントで使用。
- `_cmsBuildKToneCurve`: K（黒）チャネルのトーンカーブ構築。CMYKのBlack-Preserving変換で使用。
- これら2つはBlack-Preservingインテント（`cmscnvrt.c` の未実装部分）の依存関数。
