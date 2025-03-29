use crate::object_collection::Properties;

pub struct Tag {
    hash_index: usize,
    size: u32,
    size_disk: u32,
    name: String,
    commit_index: Option<usize>,
}

impl Tag {
    // Constructor
    pub fn new(hash_index: usize, size: u32, size_disk: u32) -> Tag {
        Tag {
            hash_index,
            size,
            size_disk,
            name: "".to_string(),
            commit_index: None,
        }
    }
    
    pub fn size_disk(&self) -> u32 {
        self.size_disk
    }

    pub fn add_name(&mut self, name: &str) {
        self.name = name.to_string();
    }

    pub fn add_commit_dep(&mut self, commit_index: &usize) {
        self.commit_index = Some(*commit_index);
    }

    // Method to display tree information
    pub fn display_info(&self) {
        println!("Hash: {}", self.hash_index);
        println!("Size: {}", self.size);
        println!("Size on Disk: {}", self.size_disk);
    }
}

impl Properties for Tag {
    fn hash_index(&self) -> &usize {
        &self.hash_index
    }

    fn set_index(&mut self, index:usize) {
        self.hash_index = index;
    }
}
