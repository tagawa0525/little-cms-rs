# cmserr.c カバレッジ

- **C ソース**: `reference/Little-CMS/src/cmserr.c`
- **Rust ファイル**: `src/context.rs`
- **概要**: エラーハンドリング・コンテキスト管理・メモリアロケータ・Mutex

## 公開API (CMSEXPORT)

| C 関数                     | Rust 対応                      | 状態                   |
| -------------------------- | ------------------------------ | ---------------------- |
| `cmsGetEncodedCMMversion`  | `crate::types::VERSION` 定数   | 実装済                 |
| `cmsstrcasecmp`            | —                              | 未実装                 |
| `cmsfilelength`            | —                              | 未実装                 |
| `_cmsMalloc`               | —                              | N/A（Rust所有権）      |
| `_cmsMallocZero`           | —                              | N/A（Rust所有権）      |
| `_cmsCalloc`               | —                              | N/A（Rust所有権）      |
| `_cmsRealloc`              | —                              | N/A（Rust所有権）      |
| `_cmsFree`                 | —                              | N/A（Rust所有権）      |
| `_cmsDupMem`               | —                              | N/A（Rust所有権）      |
| `cmsSetLogErrorHandlerTHR` | `Context::set_error_handler()` | 実装済                 |
| `cmsSetLogErrorHandler`    | —                              | 未実装（グローバル版） |
| `cmsSignalError`           | `Context::signal_error()`      | 実装済                 |
| `_cmsCreateMutex`          | —                              | N/A（Rust `Mutex`）    |
| `_cmsDestroyMutex`         | —                              | N/A（Rust `Mutex`）    |
| `_cmsLockMutex`            | —                              | N/A（Rust `Mutex`）    |
| `_cmsUnlockMutex`          | —                              | N/A（Rust `Mutex`）    |

## 内部関数

| C 関数                                | Rust 対応 | 状態              |
| ------------------------------------- | --------- | ----------------- |
| `_cmsRegisterMemHandlerPlugin`        | —         | N/A（Rust所有権） |
| `_cmsAllocMemPluginChunk`             | —         | N/A               |
| `_cmsInstallAllocFunctions`           | —         | N/A               |
| `_cmsCreateSubAlloc`                  | —         | 未実装            |
| `_cmsSubAllocDestroy`                 | —         | 未実装            |
| `_cmsSubAlloc`                        | —         | 未実装            |
| `_cmsSubAllocDup`                     | —         | 未実装            |
| `_cmsAllocLogErrorChunk`              | —         | N/A               |
| `_cmsTagSignature2String`             | —         | 未実装            |
| `_cmsAllocMutexPluginChunk`           | —         | N/A               |
| `_cmsRegisterMutexPlugin`             | —         | N/A               |
| `_cmsAllocParallelizationPluginChunk` | —         | N/A               |
| `_cmsRegisterParallelizationPlugin`   | —         | N/A               |

## 備考

- メモリ管理関数群はRustの所有権システムで不要
- Mutex関数群はRustの標準`Mutex`で代替
- SubAllocはC版のブロックアロケータ最適化。Rustでは標準アロケータを使用
- グローバル状態を持つ関数（`cmsSetLogErrorHandler`等のTHRなし版）は省略
