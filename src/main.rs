use clap::{Arg, Command};
use futures::stream::{FuturesUnordered, StreamExt};
use indicatif::{ProgressBar, ProgressStyle};
use reqwest::header::{CONTENT_LENGTH, RANGE};
use std::fs::File;
use std::io::Write;
use tokio;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let matches = Command::new("File Downloader")
        .version("1.0")
        .author("Your Name <your.email@example.com>")
        .about("Downloads a large file in parts")
        .arg(
            Arg::new("url")
                .help("The URL of the file to download")
                .required(true)
                .index(1),
        )
        .arg(
            Arg::new("part_number")
                .help("The part number to start downloading from")
                .required(false)
                .long("part_number")
                .num_args(1)
                .default_value("0"),
        )
        .arg(
            Arg::new("start_pos")
                .help("The starting byte position")
                .required(false)
                .long("start_pos")
                .num_args(1)
                .default_value("0"),
        )
        .arg(
            Arg::new("max_size")
                .help("The maximum size to download")
                .required(false)
                .long("max_size")
                .num_args(1)
                .default_value("200MB"),
        )
        .arg(
            Arg::new("part_size")
                .help("The size of each part")
                .required(false)
                .long("part_size")
                .num_args(1)
                .default_value("100MB"),
        )
        .get_matches();

    let url = matches
        .get_one::<String>("url")
        .expect("required argument 'url' not provided");

    let start_part_num: u64 = matches
        .get_one::<String>("part_number")
        .unwrap()
        .parse::<u64>()?;
    let start_pos: u64 = matches
        .get_one::<String>("start_pos")
        .unwrap()
        .parse::<u64>()?;

    let this_total_max_size = parse_size(matches.get_one::<String>("max_size").unwrap())?;
    let part_size = parse_size(matches.get_one::<String>("part_size").unwrap())?;

    println!("DOWNLOADING FILE: {}", url);

    let client = reqwest::Client::new();
    let response = client.head(url).send().await?;

    let total_file_size = match response.headers().get(CONTENT_LENGTH) {
        Some(length) => length.to_str()?.parse::<u64>()?,
        None => {
            eprintln!("Failed to retrieve the total size of the file.");
            return Ok(());
        }
    };

    println!("total_file_size: {}", total_file_size);

    let mut total_size = total_file_size.saturating_sub(start_pos);

    let is_large = if total_size > this_total_max_size {
        println!("The file size is too large to download in a single request.");
        println!("Downloading in parts instead. This may take a while.");
        total_size = this_total_max_size;
        true
    } else {
        false
    };

    let num_parts = total_size / part_size;
    let mut parts_downloaded = start_part_num;

    let tasks = FuturesUnordered::new();

    for i in 0..num_parts {
        let start = i * part_size + start_pos;
        let end = start + part_size - 1;
        let part_file = format!("part_{}", parts_downloaded + i);
        let task = download_part(url.clone(), start, Some(end), part_file);
        tasks.push(task);
    }

    // let tasks = stream::iter(0..num_parts).map(|i| {
    //     let start = i * part_size + start_pos;
    //     let end = start + part_size - 1;
    //     let part_file = format!("part_{}", parts_downloaded + i);
    //     download_part(url_string, start, Some(end), part_file)
    // }).collect::<FuturesUnordered<_>>();

    // tasks.for_each(|result| async {
    //     match result {
    //         Ok(_) => {}
    //         Err(e) => eprintln!("Error downloading part: {}", e),
    //     }
    // }).await;
    // 使用StreamExt::next来逐个获取完成的任务
    // while let Some(result) = tasks.next().await {
    //     match result {
    //         Ok(part_file) => {
    //             println!("Part {} downloaded successfully!", part_file);
    //             parts_downloaded += 1;
    //         }
    //         Err(e) => eprintln!("Error downloading file: {}", e),
    //     }
    // }
    let results: Vec<Result<_, _>> = tasks.collect().await;
    for result in results {
        match result {
            Ok(data) => {
                parts_downloaded += 1;
                println!("Downloaded successfully: {:?}", data)
            }
            Err(e) => eprintln!("Download failed: {}", e),
        }
    }

    let end_pos = total_size + start_pos;
    let remaining = total_size % part_size;
    if remaining > 0 {
        let start = num_parts * part_size + start_pos;
        let end = start + remaining;
        let part_file = format!("part_{}", parts_downloaded + num_parts);

        println!("Downloading last part from byte {} to {}...", start, end);
        download_part(url.clone(), start, Some(end), part_file).await?;
        parts_downloaded += 1;
    }

    if is_large {
        let next_start = if remaining > 0 { end_pos + 1 } else { end_pos };
        println!(
            "由于文件太大，无法一次性下载完成。请下次从 {} 处开始下载！并且设置part_numer从 {} 开始下载！",
            next_start, parts_downloaded
        );
    } else {
        println!("下载完成！");
        println!("请将所有part_文件合并为largefile文件！");
        println!("合并命令：cat part_* > largefile");
    }

    Ok(())
}

fn parse_size(size_str: &str) -> Result<u64, &'static str> {
    let size_str = size_str.to_uppercase();
    if let Some(size) = size_str.strip_suffix("GB") {
        size.trim()
            .parse::<u64>()
            .map(|s| s * 1024 * 1024 * 1024)
            .map_err(|_| "Invalid size")
    } else if let Some(size) = size_str.strip_suffix("MB") {
        size.trim()
            .parse::<u64>()
            .map(|s| s * 1024 * 1024)
            .map_err(|_| "Invalid size")
    } else {
        size_str.trim().parse::<u64>().map_err(|_| "Invalid size")
    }
}

async fn download_part(
    url: String,
    start: u64,
    end: Option<u64>,
    part_file: String,
) -> Result<String, Box<dyn std::error::Error>> {
    let client = reqwest::Client::new();
    let range = match end {
        Some(end) => format!("bytes={}-{}", start, end),
        None => format!("bytes={}-", start),
    };

    let mut response = client.get(url).header(RANGE, range).send().await?;
    let total_size = response
        .content_length()
        .ok_or("Failed to get content length")?;

    let progress_bar = ProgressBar::new(total_size);
    progress_bar.set_style(
        ProgressStyle::default_bar()
            .template("{msg} [{elapsed_precise}] {bar:40.cyan/blue} {bytes}/{total_bytes} ({eta})")
            .progress_chars("#>-"),
    );
    progress_bar.set_message(format!("Downloading {}", &part_file));

    let mut file = File::create(&part_file)?;
    while let Some(chunk) = response.chunk().await? {
        file.write_all(&chunk)?;
        progress_bar.inc(chunk.len() as u64);
    }

    progress_bar.finish_with_message(format!("Downloaded {}", &part_file));
    Ok(part_file)
}
