# cmscgats.c カバレッジ

- **C ソース**: `reference/Little-CMS/src/cmscgats.c`
- **Rust ファイル**: `src/ext/cgats.rs`
- **概要**: CGATS（色測定データ）パーサ・IT8フォーマット

## 公開API

### テーブル管理

| C 関数             | Rust 対応            | 状態           |
| ------------------ | -------------------- | -------------- |
| `cmsIT8Alloc`      | `It8::new()`         | 実装済         |
| `cmsIT8Free`       | `Drop` trait         | 実装済（暗黙） |
| `cmsIT8SetTable`   | `It8::set_table()`   | 実装済         |
| `cmsIT8TableCount` | `It8::table_count()` | 実装済         |

### シートタイプ・コメント

| C 関数               | Rust 対応               | 状態   |
| -------------------- | ----------------------- | ------ |
| `cmsIT8GetSheetType` | `It8::sheet_type()`     | 実装済 |
| `cmsIT8SetSheetType` | `It8::set_sheet_type()` | 実装済 |
| `cmsIT8SetComment`   | —                       | 未実装 |

### プロパティ

| C 関数                      | Rust 対応             | 状態                                            |
| --------------------------- | --------------------- | ----------------------------------------------- |
| `cmsIT8SetPropertyStr`      | `It8::set_property()` | 実装済                                          |
| `cmsIT8SetPropertyDbl`      | —                     | 部分的（`set_property()` で文字列として設定可） |
| `cmsIT8SetPropertyHex`      | —                     | 未実装                                          |
| `cmsIT8SetPropertyUncooked` | `It8::set_property()` | 実装済（統合）                                  |
| `cmsIT8SetPropertyMulti`    | —                     | 未実装                                          |
| `cmsIT8GetProperty`         | `It8::property()`     | 実装済                                          |
| `cmsIT8GetPropertyDbl`      | `It8::property_f64()` | 実装済                                          |
| `cmsIT8GetPropertyMulti`    | —                     | 未実装                                          |
| `cmsIT8EnumProperties`      | `It8::properties()`   | 実装済                                          |
| `cmsIT8EnumPropertyMulti`   | —                     | 未実装                                          |

### データフォーマット・データ

| C 関数                   | Rust 対応                 | 状態   |
| ------------------------ | ------------------------- | ------ |
| `cmsIT8SetDataFormat`    | `It8::set_data_format()`  | 実装済 |
| `cmsIT8EnumDataFormat`   | `It8::data_format()`      | 実装済 |
| `cmsIT8FindDataFormat`   | `It8::find_data_format()` | 実装済 |
| `cmsIT8GetDataRowCol`    | `It8::data_row_col()`     | 実装済 |
| `cmsIT8GetDataRowColDbl` | `It8::data_row_col_f64()` | 実装済 |
| `cmsIT8SetDataRowCol`    | `It8::set_data_row_col()` | 実装済 |
| `cmsIT8SetDataRowColDbl` | —                         | 未実装 |
| `cmsIT8GetData`          | `It8::data()`             | 実装済 |
| `cmsIT8GetDataDbl`       | `It8::data_f64()`         | 実装済 |
| `cmsIT8SetData`          | `It8::set_data()`         | 実装済 |
| `cmsIT8SetDataDbl`       | —                         | 未実装 |

### パッチ・ユーティリティ

| C 関数                  | Rust 対応 | 状態                             |
| ----------------------- | --------- | -------------------------------- |
| `cmsIT8GetPatchName`    | —         | 未実装                           |
| `cmsIT8GetPatchByName`  | —         | 未実装（内部`find_patch()`のみ） |
| `cmsIT8SetTableByLabel` | —         | 未実装                           |
| `cmsIT8SetIndexColumn`  | —         | 未実装                           |
| `cmsIT8DefineDblFormat` | —         | 未実装                           |

### I/O

| C 関数               | Rust 対応               | 状態                 |
| -------------------- | ----------------------- | -------------------- |
| `cmsIT8SaveToFile`   | —                       | 未実装               |
| `cmsIT8SaveToMem`    | `It8::save_to_string()` | 実装済（String出力） |
| `cmsIT8LoadFromMem`  | `It8::load_from_str()`  | 実装済               |
| `cmsIT8LoadFromFile` | —                       | 未実装               |

### Cubeファイル

| C 関数                                                                   | Rust 対応 | 状態   |
| ------------------------------------------------------------------------ | --------- | ------ |
| `cmsCreateDeviceLinkFromCubeFileTHR` / `cmsCreateDeviceLinkFromCubeFile` | —         | 未実装 |

## 備考

- CGATSの基本的なパース・データアクセス・出力は実装済。
- ファイルI/O（`cmsIT8LoadFromFile` / `cmsIT8SaveToFile`）は文字列ベースのI/Oでカバー可能。
- Multi-property、Cubeファイル対応は未実装。
