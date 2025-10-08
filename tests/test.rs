use std::error::Error as StdError;
use std::fs::{File, OpenOptions, self};
use std::env;
use std::io::Write;

use serial_test::serial;

#[serial]
#[test]
fn original_crate() -> Result<(), Box<dyn StdError>> {
    use fat16_test::Fat16;

    // fatfs クレートを使用して FatFS を初期化
    let img_path = init_fat16()?;
    let fs = Fat16::new(img_path)?;
    assert_eq!(fs.bpb.x86_jmp, [0xEB, 0x3C, 0x90]);
    assert_eq!(fs.ebpb.boot_partition_signature, [0x55, 0xAA]);

    // 読み書きしてみる
    println!("CTL: List up root directory");
    println!("-----------------------------------");
    for entry in &fs.root_dir {
        println!("{}", entry);
    }
    println!("-----------------------------------");

    println!("CTL: Read the file '1.txt'");
    println!("-----------------------------------");
    println!("{:?}", fs.read_file("1.txt")?);
    println!("-----------------------------------");

    Ok(())
}

fn init_fat16() -> Result<String, Box<dyn StdError>> {
    use fatfs::{format_volume, FileSystem as FatFs, FsOptions, FatType, FormatVolumeOptions};

    const MB: usize = 1024 * 1024;

    // 各種パス
    let out_dir = env::var("CARGO_MANIFEST_DIR")?;
    let img_file_path = format!("{}/target/tmp/fat16.img", out_dir);

    // テスト用のイメージファイルを準備
    if fs::exists(&img_file_path)? {
        fs::remove_file(&img_file_path)?;
    }
    let mut img_file = File::create(&img_file_path)?;

    // FAT16 でフォーマット
    let fmt_size = 128 * MB;
    let fmt_opts = FormatVolumeOptions::new()
        .bytes_per_sector(512)
        .total_sectors((fmt_size / 512) as u32)
        .max_root_dir_entries(512)
        .fats(2)
        .fat_type(FatType::Fat16)
        .sectors_per_track(0x20)
        .heads(0x40)
        .drive_num(0x80)
        .volume_id(0xCAFEBABE)
        .volume_label(*b"FAT16IMG   ");
    format_volume(&mut img_file, fmt_opts)?;

    // テスト用のディレクトリ・ファイルを書き込み (準備)
    let img_file = OpenOptions::new()
        .read(true)
        .write(true)
        .open(&img_file_path)?;
    let fs_opts = FsOptions::new();
    let fs = FatFs::new(img_file, fs_opts)?;

    // テスト用のディレクトリ・ファイルを書き込み
    let root_dir = fs.root_dir();
    let mut file = root_dir.create_file("1.txt")?;
    file.write_all(b"No.1\n")?;
    let mut file = root_dir.create_file("2.txt")?;
    file.write_all(b"No.2\n")?;
    let mut file = root_dir.create_file("3.txt")?;
    file.write_all(b"No.3\n")?;

    Ok(img_file_path)
}
