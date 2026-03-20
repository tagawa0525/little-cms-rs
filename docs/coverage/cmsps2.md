# cmsps2.c カバレッジ

- **C ソース**: `reference/Little-CMS/src/cmsps2.c`
- **Rust ファイル**: `src/ext/ps2.rs`
- **概要**: PostScript CSA/CRD生成

## 公開API

| C 関数                          | Rust 対応              | 状態                         |
| ------------------------------- | ---------------------- | ---------------------------- |
| `cmsGetPostScriptColorResource` | —                      | 未実装（統合ディスパッチャ） |
| `cmsGetPostScriptCRD`           | `get_postscript_crd()` | 実装済                       |
| `cmsGetPostScriptCSA`           | `get_postscript_csa()` | 実装済                       |

## 備考

- CSA（Color Space Array）とCRD（Color Rendering Dictionary）の生成は実装済。
- `cmsGetPostScriptColorResource` はCSA/CRDを統合するディスパッチャ。個別関数で直接呼び出し可能なため省略。
