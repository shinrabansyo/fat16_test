use std::fmt::Display;
use std::error::Error as StdError;

#[derive(Debug)]
pub struct FatBPB {
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

impl FatBPB {
    pub fn parse(bytes: &[u8]) -> Result<(FatBPB, &[u8]), Box<dyn StdError>> {
        let bpb = FatBPB {
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

#[derive(Debug, Clone)]
pub struct FatDirEntry {
    pub name: String,
    pub attribute: u8,
    pub reserved: u8,
    pub creation_time: FatTime,
    pub creation_date: FatDate,
    pub last_access_date: FatDate,
    pub last_modify_time: FatTime,
    pub last_modify_date: FatDate,
    pub first_cluster: u32,
    pub file_size: u32,
}

impl Display for FatDirEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // ファイル名
        write!(f, "{}", self.name)?;

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

impl FatDirEntry {
    pub fn parses(bytes: &[u8], num_entry: u16) -> Result<(Vec<FatDirEntry>, &[u8]), Box<dyn StdError>> {
        let mut entries = vec![];

        if num_entry as usize * 32 > bytes.len() {
            return Err(format!("'bytes' must be larger than {}.", num_entry * 32).into());
        }

        let mut dir_bytes = &bytes[0..(num_entry as usize * 32)];
        while dir_bytes.len() > 0 {
            match FatDirEntry::parse_entry(&dir_bytes)? {
                (Some(entry), rest) => {
                    entries.push(entry);
                    dir_bytes = rest;
                },
                (None, _) => dir_bytes = &dir_bytes[32..],
            }
        }

        Ok((entries, &bytes[(num_entry as usize * 32)..]))
    }

    pub fn parse_entry(bytes: &[u8]) -> Result<(Option<FatDirEntry>, &[u8]), Box<dyn StdError>> {
        // LFN エントリのパース
        let (lfn_name, bytes) = Self::parse_lfn(bytes)?;

        // SFN (8.3形式) エントリのパース
        let (entry, bytes) = Self::parse_sfn(bytes)?;
        let entry = match (entry, lfn_name) {
            (Some(mut entry), Some(lfn_name)) => {
                entry.name = lfn_name;
                Some(entry)
            }
            (entry, _) => entry,
        };

        Ok((entry, bytes))
    }

    fn parse_sfn(bytes: &[u8]) -> Result<(Option<FatDirEntry>, &[u8]), Box<dyn StdError>> {
        // 有効エントリの判定
        if bytes[0] == 0x00 || bytes[0] == 0xE5 {
            if bytes[0] == 0xE5 {
                println!("this is removed entry!");
            }
            return Ok((None, bytes));
        }

        // SFN エントリの読み込み
        let entry = FatDirEntry {
            name: format!("{}.{}",
                String::from_utf8_lossy(&bytes[0..8]).trim(),
                String::from_utf8_lossy(&bytes[8..11]).trim(),
            ),
            attribute: bytes[11],
            reserved: bytes[12],
            creation_time: FatTime::from((u16::from_le_bytes(bytes[14..16].try_into()?), bytes[13])),
            creation_date: FatDate::from(u16::from_le_bytes(bytes[16..18].try_into()?)),
            last_access_date: FatDate::from(u16::from_le_bytes(bytes[18..20].try_into()?)),
            last_modify_time: FatTime::from(u16::from_le_bytes(bytes[22..24].try_into()?)),
            last_modify_date: FatDate::from(u16::from_le_bytes(bytes[24..26].try_into()?)),
            first_cluster: u16::from_le_bytes(bytes[26..28].try_into()?) as u32            ,
            file_size: u32::from_le_bytes(bytes[28..32].try_into()?),
        };
        let bytes = &bytes[32..];

        Ok((Some(entry), bytes))
    }

    // READ_ONLY=0x01 HIDDEN=0x02 SYSTEM=0x04 VOLUME_ID=0x08 DIRECTORY=0x10 ARCHIVE=0x20
    // LFN=READ_ONLY|HIDDEN|SYSTEM|VOLUME_ID
    fn parse_lfn(bytes: &[u8]) -> Result<(Option<String>, &[u8]), Box<dyn StdError>> {
        // LFN 判定
        if bytes[11] != 0x0f {
            return Ok((None, bytes));
        }

        // LFN エントリが続く限り読み進める
        let mut bytes = bytes;
        let mut text = "".to_string();
        while bytes[11] == 0x0f {
            // 文字列部分の抜き取り
            let text_bytes = [
                u16::from_le_bytes(bytes[1..3].try_into()?),    // 1文字目
                u16::from_le_bytes(bytes[3..5].try_into()?),
                u16::from_le_bytes(bytes[5..7].try_into()?),
                u16::from_le_bytes(bytes[7..9].try_into()?),
                u16::from_le_bytes(bytes[9..11].try_into()?),   // 5文字目
                u16::from_le_bytes(bytes[14..16].try_into()?),  // 6文字目
                u16::from_le_bytes(bytes[16..18].try_into()?),
                u16::from_le_bytes(bytes[18..20].try_into()?),
                u16::from_le_bytes(bytes[20..22].try_into()?),
                u16::from_le_bytes(bytes[22..24].try_into()?),
                u16::from_le_bytes(bytes[24..26].try_into()?),  // 11文字目
                u16::from_le_bytes(bytes[28..30].try_into()?),  // 12文字目
                u16::from_le_bytes(bytes[30..32].try_into()?),  // 13文字目
            ];
            text = String::from_utf16(&text_bytes)? + &text;

            // 読み進める
            bytes = &bytes[32..];
        }

        // ヌル終端の除去
        let text = text.find('\0')
            .map(|idx| &text[..idx])
            .unwrap_or(&text)
            .to_string();

        Ok((Some(text), bytes))
    }
}


#[derive(Debug, Clone)]
pub struct FatDate {
    pub year: u16,
    pub month: u8,
    pub day: u8,
}

impl From<u16> for FatDate {
    fn from(date: u16) -> FatDate {
        FatDate {
            year: ((date >> 9) & 0x7F) + 1980,
            month: ((date >> 5) & 0x0F) as u8,
            day: (date & 0x1F) as u8,
        }
    }
}

#[derive(Debug, Clone)]
pub struct FatTime {
    pub hour: u8,
    pub minute: u8,
    pub second: u8,
    pub tenths_of_second: u8,
}

impl From<u16> for FatTime {
    fn from(time: u16) -> FatTime {
        FatTime::from((time, 0))
    }
}

impl From<(u16, u8)> for FatTime {
    fn from((time, tenths_of_second): (u16, u8)) -> FatTime {
        FatTime {
            hour: ((time >> 11) & 0x1F) as u8,
            minute: ((time >> 5) & 0x3F) as u8,
            second: ((time & 0x1F) * 2) as u8,
            tenths_of_second,
        }
    }
}
