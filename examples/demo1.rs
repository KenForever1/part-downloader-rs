use futures::stream::{FuturesUnordered, StreamExt};
use std::time::Duration;
use tokio::time::sleep;

// 假设这是一个异步下载函数
async fn download_file(url: &str) -> Result<String, Box<dyn std::error::Error>> {
    // 模拟下载延迟
    println!("Downloading {}...", url);
    sleep(Duration::from_secs(2)).await;
    Ok(format!("Downloaded content from {}", url))
}

#[tokio::main]
async fn main() {
    // 创建一个FuturesUnordered来管理多个异步任务
    let mut tasks = FuturesUnordered::new();

    // 需要下载的文件URL列表
    let urls = vec![
        "http://example.com/file1",
        "http://example.com/file2",
        "http://example.com/file3",
    ];

    // 为每个URL创建一个下载任务，并添加到FuturesUnordered中
    for url in urls {
        let task = download_file(url);
        tasks.push(task);
    }

    // 使用StreamExt::next来逐个获取完成的任务
    while let Some(result) = tasks.next().await {
        match result {
            Ok(content) => println!("{}", content),
            Err(e) => eprintln!("Error downloading file: {}", e),
        }
    }
}