# book

在终端中阅读本地 TXT、EPUB 和 MOBI 电子书。

## Homebrew 安装

支持 macOS 14 及以上、Apple Silicon（arm64）。安装包已编译，无需安装 Rust。

源码和安装包均已公开。安装时无需 GitHub 登录或仓库授权。

```sh
brew install KMMoonlightQ/tools/book
book
```

首次启动会生成 `~/.config/book.toml` 和默认书库目录 `~/books`，然后退出并提示配置。将书籍放入 `~/books`，或修改配置中的 `library_dir`，再次运行 `book`。

```toml
library_dir = "~/books"
```

程序会递归扫描书库中的 `.txt`、`.epub`、`.mobi` 文件。MOBI 支持未加密的 PalmDOC 格式，不支持 DRM 或全部 MOBI 压缩变体。

## 操作

| 场景 | 按键 | 操作 |
| --- | --- | --- |
| 书架 | ↑ / ↓ 或 j / k | 选择书籍 |
| 书架 | Enter | 打开书籍 |
| 书架 | q | 退出 |
| 阅读 | ↑ / ↓ | 逐行滚动 |
| 阅读 | ← / → 或 p / n | 翻页 |
| 阅读 | i | 打开章节目录 |
| 阅读 | Esc | 保存进度并返回书架 |
| 目录 | ↑ / ↓ 或 j / k | 选择章节 |
| 目录 | Enter / Esc | 跳转到选中章节 / 返回阅读 |

TXT 和 MOBI 按章节标题拆分，EPUB 按内容顺序读取章节。阅读页底部显示当前章节进度；到达章节边界后继续向前或向后翻页，可切换章节。阅读位置记录为章节和章内行号，旧版整书行号在首次折行后转换为章节位置。

阅读结束时先按 Esc 返回书架，再按 q 退出，以保存当前位置。

配置和阅读进度分别保存在 `~/.config/book.toml`、`~/.config/book-progress.toml`，升级不会覆盖这些文件。可通过 `BOOK_CONFIG_DIR` 指定独立配置目录，目录内文件名保持不变；其中的 `library_dir` 决定书库位置。

## 更新与检查

```sh
brew update
brew upgrade KMMoonlightQ/tools/book
book --version
book --help
```

## 从源码构建

```sh
cargo build --locked --release
```
