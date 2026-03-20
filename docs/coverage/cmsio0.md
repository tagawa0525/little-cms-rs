# cmsio0.c カバレッジ

- **C ソース**: `reference/Little-CMS/src/cmsio0.c`
- **Rust ファイル**: `src/profile/io.rs`
- **概要**: プロファイルI/O基盤（ファイル・メモリ・ストリーム）

## IoHandler

| C 関数                       | Rust 対応                                               | 状態           |
| ---------------------------- | ------------------------------------------------------- | -------------- |
| `cmsOpenIOhandlerFromNULL`   | `IoHandler::new_null()`                                 | 実装済         |
| `cmsOpenIOhandlerFromMem`    | `IoHandler::from_memory_read()` / `from_memory_write()` | 実装済         |
| `cmsOpenIOhandlerFromFile`   | `IoHandler::from_file_read()` / `from_file_write()`     | 実装済         |
| `cmsOpenIOhandlerFromStream` | —                                                       | 未実装         |
| `cmsCloseIOhandler`          | `Drop` trait                                            | 実装済（暗黙） |

## プロファイル作成・オープン・保存

| C 関数                                                     | Rust 対応                           | 状態                       |
| ---------------------------------------------------------- | ----------------------------------- | -------------------------- |
| `cmsGetProfileIOhandler`                                   | —                                   | 未実装（内部`pub(crate)`） |
| `cmsCreateProfilePlaceholder`                              | `Profile::new_placeholder()`        | 実装済                     |
| `cmsGetProfileContextID`                                   | —                                   | 未実装（Context系）        |
| `cmsOpenProfileFromIOhandlerTHR`                           | `Profile::open_from_io()` (private) | 実装済（内部）             |
| `cmsOpenProfileFromIOhandler2THR`                          | —                                   | 未実装                     |
| `cmsOpenProfileFromFileTHR` / `cmsOpenProfileFromFile`     | `Profile::open_file()`              | 実装済                     |
| `cmsOpenProfileFromStreamTHR` / `cmsOpenProfileFromStream` | —                                   | 未実装                     |
| `cmsOpenProfileFromMemTHR` / `cmsOpenProfileFromMem`       | `Profile::open_mem()`               | 実装済                     |
| `cmsSaveProfileToIOhandler`                                | `Profile::save_to_io()` (private)   | 実装済（内部）             |
| `cmsSaveProfileToFile`                                     | `Profile::save_to_file()`           | 実装済                     |
| `cmsSaveProfileToStream`                                   | —                                   | 未実装                     |
| `cmsSaveProfileToMem`                                      | `Profile::save_to_mem()`            | 実装済                     |
| `cmsCloseProfile`                                          | `Drop` trait                        | 実装済（暗黙）             |

## タグディレクトリ

| C 関数                   | Rust 対応                  | 状態                       |
| ------------------------ | -------------------------- | -------------------------- |
| `cmsGetTagCount`         | `Profile::tag_count()`     | 実装済                     |
| `cmsGetTagSignature`     | `Profile::tag_signature()` | 実装済                     |
| `cmsGetTagOffsetAndSize` | —                          | 未実装（内部`pub(crate)`） |
| `cmsIsTag`               | `Profile::has_tag()`       | 実装済                     |
| `cmsReadTag`             | `Profile::read_tag()`      | 実装済                     |
| `cmsWriteTag`            | `Profile::write_tag()`     | 実装済                     |
| `cmsReadRawTag`          | `Profile::read_raw_tag()`  | 実装済                     |
| `cmsWriteRawTag`         | `Profile::write_raw_tag()` | 実装済                     |
| `cmsLinkTag`             | `Profile::link_tag()`      | 実装済                     |
| `cmsTagLinkedTo`         | `Profile::tag_linked_to()` | 実装済                     |

## ヘッダアクセサ

RustではIccHeader構造体のフィールドに直接アクセス。個別のgetter/setterメソッドではなく `profile.header.xxx` 形式。

| C 関数                                                        | Rust 対応                                      | 状態                 |
| ------------------------------------------------------------- | ---------------------------------------------- | -------------------- |
| `cmsGetHeaderRenderingIntent` / `cmsSetHeaderRenderingIntent` | `profile.header.rendering_intent`              | 実装済（フィールド） |
| `cmsGetHeaderFlags` / `cmsSetHeaderFlags`                     | `profile.header.flags`                         | 実装済（フィールド） |
| `cmsGetHeaderManufacturer` / `cmsSetHeaderManufacturer`       | `profile.header.manufacturer`                  | 実装済（フィールド） |
| `cmsGetHeaderCreator`                                         | `profile.header.creator`                       | 実装済（フィールド） |
| `cmsGetHeaderModel` / `cmsSetHeaderModel`                     | `profile.header.model`                         | 実装済（フィールド） |
| `cmsGetHeaderAttributes` / `cmsSetHeaderAttributes`           | `profile.header.attributes`                    | 実装済（フィールド） |
| `cmsGetHeaderProfileID` / `cmsSetHeaderProfileID`             | `profile.header.profile_id`                    | 実装済（フィールド） |
| `cmsGetHeaderCreationDateTime`                                | `profile.header.date`                          | 実装済（フィールド） |
| `cmsGetPCS` / `cmsSetPCS`                                     | `profile.header.pcs`                           | 実装済（フィールド） |
| `cmsGetColorSpace` / `cmsSetColorSpace`                       | `profile.header.color_space`                   | 実装済（フィールド） |
| `cmsGetDeviceClass` / `cmsSetDeviceClass`                     | `profile.header.device_class`                  | 実装済（フィールド） |
| `cmsGetEncodedICCversion` / `cmsSetEncodedICCversion`         | `profile.header.version`                       | 実装済（フィールド） |
| `cmsGetProfileVersion` / `cmsSetProfileVersion`               | `Profile::version_f64()` / `set_version_f64()` | 実装済               |

## 内部関数

| C 関数               | Rust 対応                           | 状態   |
| -------------------- | ----------------------------------- | ------ |
| `_cmsSearchTag`      | `Profile::search_tag()` (private)   | 実装済 |
| `_cmsReadHeader`     | `Profile::read_header()` (private)  | 実装済 |
| `_cmsWriteHeader`    | `Profile::write_header()` (private) | 実装済 |
| `_cmsGetTagTrueType` | `Profile::tag_true_type()`          | 実装済 |

## 備考

- Stream I/O (`cmsOpenIOhandlerFromStream` 等) はファイル/メモリI/Oで十分なため省略。
- ヘッダアクセサはRustの構造体フィールドアクセスで代替（getter/setterパターン不要）。
