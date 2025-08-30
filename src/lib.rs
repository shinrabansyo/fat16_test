use std::error::Error as StdError;
use std::fs::File;
use std::path::Path;
use std::io::Read;

#[derive(Debug)]
pub struct Fat16 {
    pub bpb: Fat16BPB,
    pub ebpb: Fat16EBPB,
    pub alloc_table: Fat16AllocTable,
    pub clusters: Vec<Fat16Cluster>,
}

impl Fat16 {
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Fat16, Box<dyn StdError>> {
        // ファイルを読み込む
        let mut file = File::open(path).unwrap();
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes).unwrap();

        // FAT16 パース
        let (bpb, bytes) = Fat16BPB::parse(&bytes)?;
        let (ebpb, _bytes) = Fat16EBPB::parse(bytes)?;
        let (alloc_table, bytes) = Fat16AllocTable::parse(bytes)?;
        let (clusters, _) = Fat16Cluster::parse(bytes)?;

        Ok(Fat16 { bpb, ebpb, alloc_table, clusters })
    }
}

#[repr(C, packed)]
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
    // Total Sectors 16 (2bytes)
    pub total_sectors_16: u16,
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
            total_sectors_16: u16::from_le_bytes(bytes[19..21].try_into()?),
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

#[repr(C, packed)]
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

*/

#[derive(Debug)]
pub struct Fat16AllocTable {

}

impl Fat16AllocTable {
    pub fn parse(bytes: &[u8]) -> Result<(Fat16AllocTable, &[u8]), Box<dyn StdError>> {
        todo!()
    }
}

#[derive(Debug)]
pub struct Fat16Cluster {

}

impl Fat16Cluster {
    pub fn parse(bytes: &[u8]) -> Result<(Vec<Fat16Cluster>, &[u8]), Box<dyn StdError>> {
        todo!()
    }
}
