use clap::{Arg, Command};
use reqwest::header::CONTENT_LENGTH;
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

    for i in 0..num_parts {
        let start = i * part_size + start_pos;
        let end = start + part_size - 1;
        let part_file = format!("part_{}", parts_downloaded);

        println!(
            "Downloading part {} from byte {} to {}...",
            parts_downloaded, start, end
        );
        download_part(url, start, Some(end), &part_file).await?;
        parts_downloaded += 1;
    }

    // 最终结束的位置，下次下载的开始位置
    // todo, 如果下载过程中Cancel，则下次下载的开始位置为上个成功的part结束位置
    let end_pos = total_size + start_pos;

    let remaining = total_size % part_size;
    if remaining > 0 {
        let start = num_parts * part_size + start_pos;
        let end = start + remaining;
        let part_file = format!("part_{}", parts_downloaded);

        println!("Downloading last part from byte {} to {}...", start, end);
        download_part(url, start, Some(end), &part_file).await?;
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

// async fn download_part(
//     url: &str,
//     start: u64,
//     end: Option<u64>,
//     part_file: &str,
// ) -> Result<(), Box<dyn std::error::Error>> {
//     let client = reqwest::Client::new();
//     let range = match end {
//         Some(end) => format!("bytes={}-{}", start, end),
//         None => format!("bytes={}-", start),
//     };

//     let mut response = client.get(url).header("Range", range).send().await?;
//     let mut file = File::create(part_file)?;

//     while let Some(chunk) = response.chunk().await? {
//         file.write_all(&chunk)?;
//     }

//     Ok(())
// }

use indicatif::{ProgressBar, ProgressStyle};
use reqwest::header::RANGE;

async fn download_part(
    url: &str,
    start: u64,
    end: Option<u64>,
    part_file: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let client = reqwest::Client::new();
    let range = match end {
        Some(end) => format!("bytes={}-{}", start, end),
        None => format!("bytes={}-", start),
    };

    // Range index by 0, so 0-999, download 1000 bytes
    let mut response = client.get(url).header(RANGE, range).send().await?;
    let total_size = response
        .content_length()
        .ok_or("Failed to get content length")?;

    // 创建进度条
    let progress_bar = ProgressBar::new(total_size);
    progress_bar.set_style(
        ProgressStyle::default_bar()
            .template("{msg} [{elapsed_precise}] {bar:40.cyan/blue} {bytes}/{total_bytes} ({eta})")
            .progress_chars("#>-"),
    );
    progress_bar.set_message(format!("Downloading {}", part_file));

    let mut file = File::create(part_file)?;
    while let Some(chunk) = response.chunk().await? {
        file.write_all(&chunk)?;
        progress_bar.inc(chunk.len() as u64);
    }

    progress_bar.finish_with_message(format!("Downloaded {}", part_file));
    Ok(())
}
