use crate::object_collection::Properties;

pub struct Tree {
    hash_index: usize,
    size: u32,
    size_disk: u32,
    path: String,
    commits: Vec<usize>,
}

impl Tree {
    // Constructor
    pub fn new(hash_index: usize, size: u32, size_disk: u32) -> Self {
        Tree {
            hash_index,
            size,
            size_disk,
            path: "".to_string(),
            commits: Vec::new(),
        }
    }

    pub fn size_disk(&self) -> u32 {
        self.size_disk
    }

    pub fn path(&self) -> &str {
        &self.path
    }

    pub fn add_path(&mut self, path: &str) {
        self.path = path.to_string();
    }

    pub fn add_commit(&mut self, commit_index: &usize) {
        self.commits.push(*commit_index);
    }

    // Method to display tree information
    pub fn display_info(&self) {
        println!("Hash: {}", self.hash_index);
        println!("Size: {} meters", self.size);
        println!("Size on Disk: {} years", self.size_disk);
    }
}

impl Properties for Tree {
    fn hash_index(&self) -> &usize {
        &self.hash_index
    }
    fn set_index(&mut self, index: usize) {
        self.hash_index = index;
    }
}
