use crate::object_collection::Properties;

#[derive(Debug, Default)]
pub struct Commit {
    hash_index: usize,
    size: u32,
    size_disk: u32,
    blob_deps: Vec<usize>,
    tree_deps: Vec<usize>,
    tag_deps: Vec<usize>,
    lightweight_tags: Vec<String>,
}

impl Commit {
    // Constructor
    pub fn new(hash_index: usize, size: u32, size_disk: u32) -> Self {
        Commit {
            hash_index,
            size,
            size_disk,
            blob_deps: Vec::new(),
            tree_deps: Vec::new(),
            tag_deps: Vec::new(),
            lightweight_tags: Vec::new(),
        }
    }

    pub fn size_disk(&self) -> u32 {
        self.size_disk
    }

    pub fn add_blob_dep(&mut self, blob_index: &usize) {
        self.blob_deps.push(*blob_index);
    }

    pub fn blob_deps(&self) -> &Vec<usize> {
        &self.blob_deps
    }

    pub fn add_tree_dep(&mut self, tree_index: &usize) {
        self.tree_deps.push(*tree_index);
    }

    pub fn tree_deps(&self) -> &Vec<usize> {
        &self.tree_deps
    }

    pub fn add_tag_dep(&mut self, tag_index: &usize) {
        self.tag_deps.push(*tag_index);
    }

    pub fn tag_deps(&self) -> &Vec<usize> {
        &self.tag_deps
    }

    pub fn add_lightweight_tag(&mut self, lightweight_tag: &str) {
        self.lightweight_tags.push(lightweight_tag.to_string());
    }

    pub fn lightweight_tags(&self) -> &Vec<String> {
        &self.lightweight_tags
    }

    // Method to display tree information
    pub fn display_info(&self) {
        print!("Hash: {}", self.hash_index);
        print!(", Size: {}", self.size);
        println!(", Size on Disk: {}", self.size_disk);
    }
}

impl Properties for Commit {
    fn hash_index(&self) -> &usize {
        &self.hash_index
    }

    fn set_index(&mut self, index: usize) {
        self.hash_index = index;
    }
}
