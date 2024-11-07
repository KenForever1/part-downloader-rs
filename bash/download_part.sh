#!/bin/bash

# 本次下载文件大小 500 MB

THIS_TOTAL_MAX_SIZE=$((200 * 1024 * 1024))

# Size of each part in bytes (100MB)

PART_SIZE=$((100 * 1024 * 1024))


START_PART_NUM=0

START_POS=0

# usage: ./download_large_file.sh <URL> <part_numer> <start_pos>

if [ $# -lt 1 ]; then

    echo "Usage: $0 <URL> [part_numer] [start_pos]"

    exit 1

fi

# URL of the file to download

# URL="http://example.com/largefile"

URL=$1

echo "DOWNLOADING FILE: $URL"

# 判断是否需要从指定位置开始下载

if [ ! -z "$2" ]; then

    START_PART_NUM=$2

    echo "Starting from part_numer: $START_PART_NUM"

fi

if [ ! -z "$3" ]; then

    START_POS=$3

    echo "Starting from byte: $START_POS"

fi

# Get the total size of the file in bytes using curl

TOTAL_FILE_SIZE=$(curl -sI "$URL" | grep -i Content-Length | awk '{print $2}' | tr -d '\r')

# Check if TOTAL_SIZE was successfully retrieved

if [ -z "$TOTAL_FILE_SIZE" ]; then

    echo "Failed to retrieve the total size of the file."

    exit 1

fi

# Calculate the size of the file to download

TOTAL_SIZE=$((TOTAL_FILE_SIZE - START_POS))

IS_LARGE=FALSE

if [ $TOTAL_SIZE -gt "$THIS_TOTAL_MAX_SIZE" ]; then

    IS_LARGE=TRUE

    echo "The file size is too large to download in a single request."

    TOTAL_SIZE="$THIS_TOTAL_MAX_SIZE"

    echo "Downloading in parts instead."

    echo "This may take a while."

fi

# Calculate the number of parts

NUM_PARTS=$((TOTAL_SIZE / PART_SIZE))


# Download each part
DOWNLOADED_PART_NUM=$START_PART_NUM
for ((i = 0; i < $NUM_PARTS; i++)); do

    START=$(START_POS + (i * PART_SIZE))

    END=$((START + PART_SIZE - 1))

    PART_FILE="part_${DOWNLOADED_PART_NUM}"

    # Download the part using curl

    echo "Downloading part $DOWNLOADED_PART_NUM from byte $START to $END..."

    curl -r $START-$END -o $PART_FILE "$URL"
    DOWNLOADED_PART_NUM=$((DOWNLOADED_PART_NUM + 1))

done

# Download the last part if the total size is not a multiple of PART_SIZE

REMAINING=$((TOTAL_SIZE % PART_SIZE))
END_POS=$((TOTAL_SIZE + START_POS))
if [ $REMAINING -ne 0 ]; then

    START=$((NUM_PARTS * PART_SIZE + START_POS))


    PART_FILE="part_${DOWNLOADED_PART_NUM}"

    echo "Downloading last part from byte $START to end..."

    curl -r $START- -o $PART_FILE "$URL"

    DOWNLOADED_PART_NUM=$((DOWNLOADED_PART_NUM + 1))

fi

if [ "$IS_LARGE" = "TRUE" ]; then

    if [ $REMAINING -ne 0 ]; then
        echo "由于文件太大，无法一次性下载完成。请下次从${END_POS}处开始下载！并且设置part_numer从${DOWNLOADED_PART_NUM}开始下载！"
    else
        echo "由于文件太大，无法一次性下载完成。请下次从${END_POS + 1}处开始下载！并且设置part_numer从${DOWNLOADED_PART_NUM}开始下载！"
    fi
else

    echo "下载完成！"

    echo "请将所有part_文件合并为largefile文件！"

    echo "合并命令：cat part_* > largefile"

fi

# # Combine all parts into a single file

# echo "Combining parts into final file..."

# cat part_* > largefile

# # Clean up part files

# echo "Cleaning up part files..."

# rm part_*

# echo "Download and merge complete!"

# verify with md5sum 验证文件是否下载完整
