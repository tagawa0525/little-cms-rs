# cmsnamed.c カバレッジ

- **C ソース**: `reference/Little-CMS/src/cmsnamed.c`
- **Rust ファイル**: `src/pipeline/named.rs`
- **概要**: 名前付きカラーリスト・MLU（多言語Unicode）・辞書・プロファイルシーケンス

## 公開API

### MLU（多言語Unicode）

| C 関数                    | Rust 対応                    | 状態                      |
| ------------------------- | ---------------------------- | ------------------------- |
| `cmsMLUalloc`             | `Mlu::new()`                 | 実装済                    |
| `cmsMLUsetASCII`          | `Mlu::set_ascii()`           | 実装済                    |
| `cmsMLUsetUTF8`           | `Mlu::set_utf8()`            | 実装済                    |
| `cmsMLUsetWide`           | —                            | 未実装（UTF-8 APIで代替） |
| `cmsMLUdup`               | `Clone` trait                | 実装済（暗黙）            |
| `cmsMLUfree`              | `Drop` trait                 | 実装済（暗黙）            |
| `cmsMLUgetASCII`          | `Mlu::get_ascii()`           | 実装済                    |
| `cmsMLUgetUTF8`           | `Mlu::get_utf8()`            | 実装済                    |
| `cmsMLUgetWide`           | —                            | 未実装（UTF-8 APIで代替） |
| `cmsMLUgetTranslation`    | `Mlu::find_best()` (private) | 実装済（内部メソッド）    |
| `cmsMLUtranslationsCount` | `Mlu::translations_count()`  | 実装済                    |
| `cmsMLUtranslationsCodes` | `Mlu::translation_codes()`   | 実装済                    |

### 名前付きカラー

| C 関数                     | Rust 対応                  | 状態           |
| -------------------------- | -------------------------- | -------------- |
| `cmsAllocNamedColorList`   | `NamedColorList::new()`    | 実装済         |
| `cmsFreeNamedColorList`    | `Drop` trait               | 実装済（暗黙） |
| `cmsDupNamedColorList`     | `Clone` trait              | 実装済（暗黙） |
| `cmsAppendNamedColor`      | `NamedColorList::append()` | 実装済         |
| `cmsNamedColorCount`       | `NamedColorList::count()`  | 実装済         |
| `cmsNamedColorInfo`        | `NamedColorList::info()`   | 実装済         |
| `cmsNamedColorIndex`       | `NamedColorList::find()`   | 実装済         |
| `_cmsStageAllocNamedColor` | —                          | 未実装         |
| `cmsGetNamedColorList`     | —                          | 未実装         |

### プロファイルシーケンス

| C 関数                               | Rust 対応                    | 状態           |
| ------------------------------------ | ---------------------------- | -------------- |
| `cmsAllocProfileSequenceDescription` | `ProfileSequenceDesc::new()` | 実装済         |
| `cmsFreeProfileSequenceDescription`  | `Drop` trait                 | 実装済（暗黙） |
| `cmsDupProfileSequenceDescription`   | `Clone` trait                | 実装済（暗黙） |

### 辞書

| C 関数                | Rust 対応                           | 状態           |
| --------------------- | ----------------------------------- | -------------- |
| `cmsDictAlloc`        | `Dict::new()`                       | 実装済         |
| `cmsDictFree`         | `Drop` trait                        | 実装済（暗黙） |
| `cmsDictAddEntry`     | `Dict::add()`                       | 実装済         |
| `cmsDictDup`          | `Clone` trait                       | 実装済（暗黙） |
| `cmsDictGetEntryList` | `Dict::iter()`                      | 実装済         |
| `cmsDictNextEntry`    | イテレータパターン (`Dict::iter()`) | 実装済         |

## 備考

- Wide文字列関数（`cmsMLUsetWide`/`cmsMLUgetWide`）はプラットフォーム依存の`wchar_t`を扱うC固有の機能。RustではUTF-8ベースのAPIで統一。
- `_cmsStageAllocNamedColor`: 名前付きカラーをパイプラインステージとして割り当てる関数。未実装。
- `cmsGetNamedColorList`: Transform内のNamedColorListを取得する関数。未実装。
