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

