use std::error::Error as StdError;
use std::fs::File;
use std::path::Path as StdPath;
use std::io::Read;

use crate::fat::{FatBPB, FatDirEntry};
use crate::utils::Path as MyPath;

#[derive(Debug)]
pub struct Fat16 {
    pub bpb: FatBPB,
    pub ebpb: Fat16EBPB,
    pub alloc_table: Fat16AllocTable,
    pub root_dir: Vec<FatDirEntry>,
    pub clusters: Vec<u8>,
}

impl Fat16 {
    pub fn new<P: AsRef<StdPath>>(path: P) -> Result<Fat16, Box<dyn StdError>> {
        // ファイルを読み込む
        let mut file = File::open(path).unwrap();
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes).unwrap();

        // FAT16 パース
        let (bpb, bytes) = FatBPB::parse(&bytes)?;
        let (ebpb, bytes) = Fat16EBPB::parse(&bytes)?;
        let (alloc_table, bytes) = Fat16AllocTable::parse(&bytes, &bpb)?;

        // root_dir_sectors = ((fat_boot->root_entry_count * 32) + (fat_boot->bytes_per_sector - 1)) / fat_boot->bytes_per_sector;

        // Root Directory をパース
        let (root_dir, bytes) = FatDirEntry::parses(&bytes, bpb.root_entry_count)?;

        Ok(Fat16 { bpb, ebpb, alloc_table, root_dir, clusters: bytes.to_vec() })
    }

    pub fn read_file(&self, path: &MyPath) -> Result<Vec<u8>, Box<dyn StdError>> {
        // path にマッチする DirEntry を探す
        let entry = self.find_dir_entry(path)?;

        // FAT テーブルの参照
        // クラスタを辿ってデータを取得
        let cluster_chain = self.alloc_table.get_cluster_chain(entry.first_cluster as u16);
        let mut file = Vec::new();
        for cluster_number in cluster_chain {
            let cluster_data = self.read_cluster(cluster_number)?;
            file.extend(cluster_data);
        }
        file.truncate(entry.file_size as usize);

        Ok(file)
    }

    pub fn read_directory(&self, path: &MyPath) -> Result<Vec<FatDirEntry>, Box<dyn StdError>> {
        // path にマッチする DirEntry を探す
        let entry = self.find_dir_entry(path)?;
        self.read_dir_entry(&entry)
    }

    fn read_cluster<'a>(&'a self, cluster_number: u16) -> Result<&'a [u8], Box<dyn StdError>> {
        // (B / S) * (S / C)
        // B / C
        let bytes_per_cluster = self.bpb.bytes_per_sector as usize * self.bpb.sectors_per_cluster as usize;
        let head = (cluster_number as usize - 2) * bytes_per_cluster;

        // 範囲チェック
        if head + bytes_per_cluster > self.clusters.len() {
            return Err(format!("Cluster number out of range. len = {}", self.clusters.len()).into());
        }

        Ok(&self.clusters[head..head + bytes_per_cluster])
    }

    fn find_dir_entry(&self, path: &MyPath) -> Result<FatDirEntry, Box<dyn StdError>> {
        // path にマッチする DirEntry を探す
        let dirs = path.parse();

        let mut entry = self.root_dir.clone();
        for dir in &dirs[..dirs.len()-1] {
            let d = entry
                .iter()
                .find(|e| &e.name.to_ascii_lowercase() == dir)
                .ok_or("No such file or direcotry")?;
            entry = self.read_dir_entry(d)?;
        }

        entry
            .into_iter()
            .find(|e| &e.name.to_ascii_lowercase() == dirs[dirs.len()-1])
            .ok_or("No such file or direcotry".into())
    }

    fn read_dir_entry(&self, dir_entry: &FatDirEntry) -> Result<Vec<FatDirEntry>, Box<dyn StdError>> {
        // FAT テーブルの参照
        // クラスタを辿ってデータを取得
        let bytes_per_cluster = self.bpb.bytes_per_sector as usize * self.bpb.sectors_per_cluster as usize;
        let bytes_per_entry = 32;
        let entries_per_cluster = (bytes_per_cluster / bytes_per_entry) as u16;

        let cluster_chain = self.alloc_table.get_cluster_chain(dir_entry.first_cluster as u16);
        let mut dirs = Vec::new();
        for cluster_number in cluster_chain {
            let cluster_data = self.read_cluster(cluster_number)?;
            let (part_of_dirs, _) = FatDirEntry::parses(cluster_data, entries_per_cluster)?;
            dirs.extend(part_of_dirs);
        }

        Ok(dirs)
    }
}


#[derive(Debug)]
pub struct Fat16EBPB {
    // Drive Number (1byte)
    pub drive_number: u8,
    // Reserved1 (1byte)
    pub reserved1: u8,
    // Boot Signature (1byte)
    pub boot_signature: u8,
    // Volume ID (4bytes)
    pub volume_id: u32,
    // Volume Label (11bytes)
    pub volume_label: [u8; 11],
    // File System Type (8bytes)
    pub file_system_type: [u8; 8],
    // Boot Code (448bytes)
    pub boot_code: [u8; 448],
    // Boot Partition Signature (2bytes)
    pub boot_partition_signature: [u8; 2],
}

impl Fat16EBPB {
    pub fn parse(bytes: &[u8]) -> Result<(Fat16EBPB, &[u8]), Box<dyn StdError>> {
        let ebpb = Fat16EBPB {
            drive_number: bytes[0],
            reserved1: bytes[1],
            boot_signature: bytes[2],
            volume_id: u32::from_le_bytes(bytes[3..7].try_into()?),
            volume_label: bytes[7..18].try_into()?,
            file_system_type: bytes[18..26].try_into()?,
            boot_code: bytes[26..474].try_into()?,
            boot_partition_signature: bytes[474..476].try_into()?,
        };
        Ok((ebpb, &bytes[476..]))
    }
}

/*

1. dir entry の cluster number を取得
2. FAT[cluster number] から次の cluster number を取得
3. cluster number が 0xFFF8 以上になるまで繰り返す(EOFまで)
4. dir entry の attribute が directory=0x10 ならクラスタの中身はディレクトリ
5. archive=0x20 ならファイル


FAT の先頭アドレス
BPB + EBPB

FAT 領域のサイズ
num_fats * sectors_per_fat * bytes_per_sector

FAT のエントリ数(クラスタ数)
total_sectors / sectors_per_cluster

root directory の先頭アドレス
BPB + EBPB + FAT 領域のサイズ

*/

#[derive(Debug)]
pub struct Fat16AllocTable {
    table: Vec<u16>,
}

impl Fat16AllocTable {
    pub fn parse<'a>(bytes: &'a [u8], bpb: &FatBPB) -> Result<(Fat16AllocTable, &'a [u8]), Box<dyn StdError>> {
        // u32 キャスト
        let num_fats = bpb.num_fats as u32;
        let sectors_per_fat = bpb.sectors_per_fat as u32;
        let bytes_per_sector = bpb.bytes_per_sector as u32;
        let total_sectors = bpb.total_sectors as u32;
        let sectors_per_cluster = bpb.sectors_per_cluster as u32;

        // 領域サイズなどを計算
        let fat_size = num_fats * sectors_per_fat * bytes_per_sector;

        let fat_entry_cnt = if total_sectors == 0 { // total_sectors が 0 の場合、セクタ数は65536以上。 large_sectors を使う
            bpb.large_sectors / sectors_per_cluster
        } else {
            total_sectors / sectors_per_cluster
        };

        // FAT エントリを読み込み
        let mut table = vec![];
        for id in 0..fat_entry_cnt {
            let offset = (id * 2) as usize;
            let entry = u16::from_le_bytes(bytes[offset..offset+2].try_into()?);
            table.push(entry);
        }

        Ok((Fat16AllocTable { table }, &bytes[fat_size as usize..]))
    }

    pub fn get_cluster_chain(&self, start_cluster: u16) -> Vec<u16> {
        let mut chain = vec![];
        let mut cluster = start_cluster;

        while cluster < 0xFFF8 {
            chain.push(cluster);
            cluster = self.table[cluster as usize];
        }

        chain
    }
}
