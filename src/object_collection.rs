use crate::blob::*;
use crate::commit::*;
use crate::tag::*;
use crate::tree::*;
use std::collections::HashMap;
use std::sync::RwLock;

pub trait Properties {
    fn hash_index(&self) -> &usize;
    fn set_index(&mut self, index:usize);
}

pub struct BasicObjectContainer<T> {
    items: Vec<RwLock<T>>,
    lookup: HashMap<String, usize>,
}

impl<T> BasicObjectContainer<T>
where
    T: Properties,
{
    pub fn new() -> Self {
        BasicObjectContainer {
            items: Vec::new(),
            lookup: HashMap::new(),
        }
    }

    pub fn add(&mut self, hash: &str, object: T) {
        let index = object.hash_index().clone();
        self.items.push(RwLock::new(object));
        self.lookup.insert(hash.to_string(), index);
    }

    pub fn get_index(&self, hash: &str) -> Option<&usize> {
        self.lookup.get(hash)
    }

    pub fn get(&self, hash: &str) -> &RwLock<T> {
        let index = self.lookup[hash];
        &self.items[index]
    }

    pub fn get_by_index(&self, index: &usize) -> &RwLock<T> {
        &self.items[*index]
    }

    pub fn count(&self) -> usize {
        self.items.len()
    }

    pub fn object_hash_iter(&self) -> impl Iterator<Item = (&String, &usize)> {
        self.lookup.iter()
    }

    pub fn object_iter(&self) -> impl Iterator<Item = &RwLock<T>> {
        self.items.iter()
    }

    // Note that this is slow and should not be done in a loop.
    pub fn lookup_hash_for_index(&self, index: &usize) -> Option<&String> {
        let hash = self.lookup.iter().find_map(|(key, &val)| if val == *index { Some(key) } else { None });
        hash
    }

}

impl<T> Default for BasicObjectContainer<T>
where T: Properties {
    fn default() -> Self {
        Self::new()
    }
}

pub struct ObjectContainer {
    commits: BasicObjectContainer<Commit>,
    blobs: BasicObjectContainer<Blob>,
    trees: BasicObjectContainer<Tree>,
    tags: BasicObjectContainer<Tag>,
}

impl ObjectContainer {
    pub fn new() -> Self {
        ObjectContainer {
            commits: BasicObjectContainer::new(),
            blobs: BasicObjectContainer::new(),
            trees: BasicObjectContainer::new(),
            tags: BasicObjectContainer::new(),
        }
    }

    pub fn mut_commits(&mut self) -> &mut BasicObjectContainer<Commit> {
        &mut self.commits
    }

    pub fn commits(&self) -> &BasicObjectContainer<Commit> {
        &self.commits
    }

    pub fn mut_blobs(&mut self) -> &mut BasicObjectContainer<Blob> {
        &mut self.blobs
    }

    pub fn blobs(&self) -> &BasicObjectContainer<Blob> {
        &self.blobs
    }

    pub fn mut_trees(&mut self) -> &mut BasicObjectContainer<Tree> {
        &mut self.trees
    }

    pub fn trees(&self) -> &BasicObjectContainer<Tree> {
        &self.trees
    }

    pub fn mut_tags(&mut self) -> &mut BasicObjectContainer<Tag> {
        &mut self.tags
    }

    pub fn tags(&self) -> &BasicObjectContainer<Tag> {
        &self.tags
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug)]
    struct MockObject {
        index: usize,
    }

    impl Properties for MockObject {
        fn hash_index(&self) -> &usize {
            &self.index
        }

        fn set_index(&mut self, index: usize) {
            self.index = index;
        }
    }

    #[test]
    fn test_basic_object_container_add_and_get() {
        let mut container = BasicObjectContainer::new();
        let hash = "mock_hash";
        let object = MockObject { index: 0 };

        container.add(hash, object);

        assert_eq!(container.count(), 1);
        assert!(container.get_index(hash).is_some());
        assert_eq!(container.get_index(hash).unwrap(), &0);
    }

    #[test]
    fn test_basic_object_container_get_by_index() {
        let mut container = BasicObjectContainer::new();
        let hash = "mock_hash";
        let object = MockObject { index: 0 };

        container.add(hash, object);

        let retrieved_object = container.get_by_index(&0).read().unwrap();
        assert_eq!(retrieved_object.hash_index(), &0);
    }

    #[test]
    fn test_basic_object_container_lookup_hash_for_index() {
        let mut container = BasicObjectContainer::new();
        let hash = "mock_hash";
        let object = MockObject { index: 0 };

        container.add(hash, object);

        let retrieved_hash = container.lookup_hash_for_index(&0);
        assert!(retrieved_hash.is_some());
        assert_eq!(retrieved_hash.unwrap(), hash);
    }

    #[test]
    fn test_object_container_add_commit() {
        let mut container = ObjectContainer::new();
        let hash = "mock_commit_hash";
        let commit = Commit::new(0, 123, 456);

        container.mut_commits().add(hash, commit);

        assert_eq!(container.commits().count(), 1);
        assert!(container.commits().get_index(hash).is_some());
    }

    #[test]
    fn test_object_container_retrieve_commit() {
        let mut container = ObjectContainer::new();
        let hash = "mock_commit_hash";
        let commit = Commit::new(0, 123, 456);

        container.mut_commits().add(hash, commit);

        let retrieved_commit = container.commits().get(hash).read().unwrap();
        assert_eq!(retrieved_commit.hash_index(), &0);
    }

    #[test]
    fn test_object_container_lookup_commit_hash() {
        let mut container = ObjectContainer::new();
        let hash = "mock_commit_hash";
        let commit = Commit::new(0, 123, 456);

        container.mut_commits().add(hash, commit);

        let retrieved_hash = container.commits().lookup_hash_for_index(&0);
        assert!(retrieved_hash.is_some());
        assert_eq!(retrieved_hash.unwrap(), hash);
    }
}
