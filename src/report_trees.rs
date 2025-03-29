use crate::object_collection::{ObjectContainer, Properties};
use crate::utils::display_size;
use std::{
    collections::HashMap,
    time::Instant,
};

pub fn report_trees(container: &ObjectContainer) {

    println!("Building tree report...");
    let start = Instant::now();
    let mut total_size:u64 = 0;
    let mut largest_tree_size:u32 = 0;
    let mut largest_tree_index:usize = 0;
    let mut tree_collector:HashMap<String, Vec<usize>> = HashMap::new();

    for rw_tree in container.trees().object_iter() {
        let tree = rw_tree.read().unwrap();
        total_size += tree.size_disk() as u64;
        if largest_tree_size < tree.size_disk() {
            largest_tree_size = tree.size_disk();
            largest_tree_index = *tree.hash_index();
        }

        match tree_collector.get_mut(tree.path()) {
            Some(trees) => {
                trees.push(*tree.hash_index());
            },
            None => {
                tree_collector.insert(tree.path().to_string(), vec![*tree.hash_index()]);
            }
        }
    }

    // Calculate most_trees.
    let mut most_trees_at_path_count: usize = 0;
    let mut most_trees_at_path:String = String::new();
    let mut most_trees_at_path_total_size:u64 = 0;

    for (path, trees) in &tree_collector {
        if most_trees_at_path_count < trees.len() {
            most_trees_at_path_count = trees.len();
            most_trees_at_path = path.clone();
        }
    }

    let trees = tree_collector.get(&most_trees_at_path).unwrap();
    for tree_index in trees {
        let tree = container.trees().get_by_index(tree_index);
        most_trees_at_path_total_size += tree.read().unwrap().size_disk() as u64;
    }

    println!();
    println!("Tree Report");
    println!("-------------------------------------------------------");
    println!("Total Trees: {}", container.trees().count());
    println!("Total Trees Size: {}",display_size(total_size));
    println!("Largest Tree Object Size: {}", display_size(largest_tree_size as u64));
    println!("Largest Tree Object Id: {}", container.commits().lookup_hash_for_index(&largest_tree_index).unwrap());
    println!("Most Trees at Path: {}", most_trees_at_path);
    println!("Count Most Trees at Path: {}", most_trees_at_path_count);
    println!("Most Trees at Path Total Size: {}\n\n", display_size(most_trees_at_path_total_size));
    println!("Tree report created in: {:?}", start.elapsed());

}

