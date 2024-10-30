## 简介

这是一个采用rust实现的一个命令行工具。将大文件分成多个part下载，然后再合并文件。

应用场景：
比如你有个跳板机，有个开发机。开发机磁盘空间很大（很多个TB），但是下载速度很慢（1MB/s）。跳板机的下载速度很快，但是磁盘空间很小，比如只有50GB。
这时，你有一个100GB的文件需要下载到开发机，你不想漫长的等待。那么可以用此工具在跳板机上下载多个part文件，比如part1, part2, part3 ...

然后将part文件scp到开发机，等所有part都下载完成后，使用cat命令合并文件，并校验md5值是否正确。

实现原理，采用了HTTP请求中的RANGE Header指定范围。curl也可以实现同样的功能，但是需要自己编写脚本计算Range。

## 用法

```bash
cargo run -- http://example.com/largefile --max_size "500MB" --part_size "50MB"
```

```bash
down-part http://example.com/largefile --max_size 20GB --part_size 10GB
```
下载成功后会提示你，
```
由于文件太大，无法一次性下载完成。请下次从 11111111 处开始下载！并且设置part_numer从 3 开始下载！
```
接着上次的位置下载,

```bash
down-part http://example.com/largefile --max_size 20GB --part_size 10GB --start_pos 11111111 --part_number 3
```
后续part下载，可以更改max_size和part_size参数，不用和第一次保持一致。

```
下载完成！
请将所有part_文件合并为largefile文件！
合并命令：cat part_* > largefile
```

## 编译静态链接可执行文件

为了更好的分发可执行文件，可以采用静态链接的方式编译。

方法1 x86_64-unknown-linux-gnu
```bash
RUSTFLAGS="-C target-feature=+crt-static" cargo build --release --target x86_64-unknown-linux-gnu
```

方法2 x86_64-unknown-linux-musl, 目前方法2会编译出来的可执行程序，会运行段错误，暂未解决
```bash
# Cross compile to musl target:
rustup target add x86_64-unknown-linux-musl
# Needs `musl-gcc` package to successfully build, `musl-tools` provides `musl-gcc` command + `musl-dev` package:
apt update && apt install musl-tools
cargo build --release --target x86_64-unknown-linux-musl

segment_err: https://github.com/rust-lang/rust/issues/95926
```

## 同步版本

examples/down_file_sync.rs

## 异步下载多个文件

异步版本是在同步基础上实现的，采用异步下载多个文件，可以提高下载的速度，实现原理：
使用 futures::stream::FuturesUnordered 来处理并发任务。
使用 stream::iter 创建一个任务流，并在每个任务中调用 download_part 函数。
使用 for_each 方法来处理每个下载任务的结果。


## 合并part文件
在使用 cat 命令合并文件时，如果你的文件是按特定命名规则（例如 part_1, part_2, … , part_10, part_11）生成的，你需要确保文件按照正确的顺序进行合并。在这种情况下，简单地使用通配符（如 part_*）可能会导致错误的排序，因为字符串排序通常是按字典顺序进行的，会将 part_10 排在 part_2 之前。
```bash
ls part_* | sort -V | xargs cat > combined_file
```