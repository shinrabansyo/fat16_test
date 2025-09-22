use std::error::Error as StdError;
use std::fs::File;
use std::path::Path;
use std::io::Read;
use std::fmt::Display;

#[derive(Debug)]
pub struct Fat16 {
    pub bpb: Fat16BPB,
    pub ebpb: Fat16EBPB,
    pub alloc_table: Fat16AllocTable,
}

impl Fat16 {
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Fat16, Box<dyn StdError>> {
        // ファイルを読み込む
        let mut file = File::open(path).unwrap();
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes).unwrap();

        // FAT16 パース
        let (bpb, bytes) = Fat16BPB::parse(&bytes)?;
        let (ebpb, bytes) = Fat16EBPB::parse(&bytes)?;
        let (alloc_table, bytes) = Fat16AllocTable::parse(&bytes, &bpb)?;

        // Root Directory をパース
        let root_dir = Fat16Dir::parse(&bytes[0..(bpb.root_entry_count as usize)])?;
        println!("{}", root_dir);

        Ok(Fat16 { bpb, ebpb, alloc_table })
    }
}

#[derive(Debug)]
pub struct Fat16BPB {
    // The first three bytes 'E8 3C 90' (3bytes)
    pub x86_jmp: [u8; 3],
    // OEM Identifier (8bytes)
    pub oem_name: [u8; 8],
    // Bytes per Sector (2bytes)
    pub bytes_per_sector: u16,
    // Sectors per Cluster (1byte)
    pub sectors_per_cluster: u8,
    // Reserved Sector Count (2bytes)
    pub reserved_sector_count: u16,
    // Number of FATs (1byte)
    pub num_fats: u8,
    // Root Entry Count (2bytes)
    pub root_entry_count: u16,
    // Total Sectors (2bytes)
    pub total_sectors: u16,
    // Media (1byte)
    pub media: u8,
    // Sectors per FAT (2bytes)
    pub sectors_per_fat: u16,
    // Sectors per Track (2bytes)
    pub sectors_per_track: u16,
    // Number of Heads (2bytes)
    pub num_heads: u16,
    // Hidden Sectors (4bytes)
    pub hidden_sectors: u32,
    // Large sector count (4bytes)
    pub large_sectors: u32,
}

impl Fat16BPB {
    pub fn parse(bytes: &[u8]) -> Result<(Fat16BPB, &[u8]), Box<dyn StdError>> {
        let bpb = Fat16BPB {
            x86_jmp: bytes[0..3].try_into()?,
            oem_name: bytes[3..11].try_into()?,
            bytes_per_sector: u16::from_le_bytes(bytes[11..13].try_into()?),
            sectors_per_cluster: bytes[13],
            reserved_sector_count: u16::from_le_bytes(bytes[14..16].try_into()?),
            num_fats: bytes[16],
            root_entry_count: u16::from_le_bytes(bytes[17..19].try_into()?),
            total_sectors: u16::from_le_bytes(bytes[19..21].try_into()?),
            media: bytes[21],
            sectors_per_fat: u16::from_le_bytes(bytes[22..24].try_into()?),
            sectors_per_track: u16::from_le_bytes(bytes[24..26].try_into()?),
            num_heads: u16::from_le_bytes(bytes[26..28].try_into()?),
            hidden_sectors: u32::from_le_bytes(bytes[28..32].try_into()?),
            large_sectors: u32::from_le_bytes(bytes[32..36].try_into()?),
        };
        Ok((bpb, &bytes[36..]))
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
    pub fn parse<'a>(bytes: &'a [u8], bpb: &Fat16BPB) -> Result<(Fat16AllocTable, &'a [u8]), Box<dyn StdError>> {
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
}

#[derive(Debug)]
pub struct Fat16DirEntry {
    filename: [u8; 8],
    ext: [u8; 3],
    attribute: u8,
    reserved: u8,
    creation_time: Fat16Time,
    creation_date: Fat16Date,
    last_access_date: Fat16Date,
    last_modify_time: Fat16Time,
    last_modify_date: Fat16Date,
    first_cluster: u32,
    file_size: u32,
}

impl Display for Fat16DirEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // ファイル名
        let filename = String::from_utf8_lossy(&self.filename);
        let ext = String::from_utf8_lossy(&self.ext);
        write!(f, "{}.{}", filename, ext)?;

        // 属性
        write!(f, " (attr: ")?;
        if self.attribute & 0x01 != 0 { write!(f, "R")?; }
        if self.attribute & 0x02 != 0 { write!(f, "H")?; }
        if self.attribute & 0x04 != 0 { write!(f, "S")?; }
        if self.attribute & 0x08 != 0 { write!(f, "V")?; }
        if self.attribute & 0x10 != 0 { write!(f, "D")?; }
        if self.attribute & 0x20 != 0 { write!(f, "A")?; }
        write!(f, ", ")?;

        // その他
        write!(f, "size: {}, first_cluster: {})", self.file_size, self.first_cluster)
    }
}

impl Fat16DirEntry {
    pub fn parse(bytes: &[u8]) -> Result<(Fat16DirEntry, &[u8]), Box<dyn StdError>> {
        // 8.3 形式
        let entry = Fat16DirEntry {
            filename: bytes[0..8].try_into()?,
            ext: bytes[8..11].try_into()?,
            attribute: bytes[11],
            reserved: bytes[12],
            creation_time: Fat16Time::from((u16::from_le_bytes(bytes[14..16].try_into()?), bytes[13])),
            creation_date: Fat16Date::from(u16::from_le_bytes(bytes[16..18].try_into()?)),
            last_access_date: Fat16Date::from(u16::from_le_bytes(bytes[18..20].try_into()?)),
            last_modify_time: Fat16Time::from(u16::from_le_bytes(bytes[22..24].try_into()?)),
            last_modify_date: Fat16Date::from(u16::from_le_bytes(bytes[24..26].try_into()?)),
            first_cluster: u16::from_le_bytes(bytes[26..28].try_into()?) as u32            ,
            file_size: u32::from_le_bytes(bytes[28..32].try_into()?),
        };
        let bytes = &bytes[32..];

        // LFN エントリ
        // if entry.attribute == 0x0F {
        //     todo!();
        // }

        Ok((entry, bytes))
    }
}

#[derive(Debug)]
pub enum Fat16Dir {
    Entries(Vec<Fat16DirEntry>),
    File(Vec<u8>),
}

impl Display for Fat16Dir {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Fat16Dir::Entries(entries) => {
                for entry in entries {
                    writeln!(f, "{}", entry)?;
                }
                Ok(())
            },
            Fat16Dir::File(data) => write!(f, "{:?}", data),
        }
    }
}

impl Fat16Dir {
    pub fn parse(bytes: &[u8]) -> Result<Fat16Dir, Box<dyn StdError>> {
        let mut entries = vec![];

        if bytes.len() % 32 != 0 {
            return Err("Directory size is not multiple of 32".into());
        }

        for i in 0..(bytes.len() / 32) {
            let (entry, _) = Fat16DirEntry::parse(&bytes[i*32..(i+1)*32])?;
            if entry.filename[0] != 0x00 && entry.filename[0] != 0xE5 { // 未使用エントリ
                entries.push(entry);
            }
        }

        Ok(Fat16Dir::Entries(entries))
    }
}

#[derive(Debug)]
pub struct Fat16Date {
    pub year: u16,
    pub month: u8,
    pub day: u8,
}

impl From<u16> for Fat16Date {
    fn from(date: u16) -> Fat16Date {
        Fat16Date {
            year: ((date >> 9) & 0x7F) + 1980,
            month: ((date >> 5) & 0x0F) as u8,
            day: (date & 0x1F) as u8,
        }
    }
}

#[derive(Debug)]
pub struct Fat16Time {
    pub hour: u8,
    pub minute: u8,
    pub second: u8,
    pub tenths_of_second: u8,
}

impl From<u16> for Fat16Time {
    fn from(time: u16) -> Fat16Time {
        Fat16Time::from((time, 0))
    }
}

impl From<(u16, u8)> for Fat16Time {
    fn from((time, tenths_of_second): (u16, u8)) -> Fat16Time {
        Fat16Time {
            hour: ((time >> 11) & 0x1F) as u8,
            minute: ((time >> 5) & 0x3F) as u8,
            second: ((time & 0x1F) * 2) as u8,
            tenths_of_second,
        }
    }
}
