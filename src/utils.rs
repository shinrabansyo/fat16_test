#[derive(Debug)]
pub struct Path {
    abs_path: String,
}

impl From<&str> for Path {
    fn from(s: &str) -> Path {
        Path { abs_path: s.to_string().to_ascii_lowercase() }
    }
}

impl Path {
    pub fn parse(&self) -> Vec<&str> {
        self.abs_path[1..].split('/').into_iter().collect()
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
