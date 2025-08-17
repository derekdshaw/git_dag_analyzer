use crate::commit::Commit;
use crate::object_collection::{ObjectContainer, Properties};
use crate::utils::display_size;
use std::{sync::RwLockReadGuard, time::Instant};

pub fn report_commits(container: &ObjectContainer) {
    println!("Building tree report...");
    let start = Instant::now();

    let mut total_size: u64 = 0;
    let mut largest_commit_size: u32 = 0;
    let mut largest_commmit_index: usize = 0;
    let mut largest_contributing_size: u64 = 0;
    let mut largest_contributing_commit_index: usize = 0;
    for rw_commit in container.commits().object_iter() {
        let commit = rw_commit.read().unwrap();
        total_size += commit.size_disk() as u64;
        if largest_commit_size < commit.size_disk() {
            largest_commit_size = commit.size_disk();
            largest_commmit_index = *commit.hash_index();
        }

        // maybe store back in commit?
        let contributing = calc_commit_size(&commit, container);
        if largest_contributing_size < contributing {
            largest_contributing_size = contributing;
            largest_contributing_commit_index = *commit.hash_index();
        }
    }

    println!();
    println!("Commit Report");
    println!("-------------------------------------------------------");
    println!("Total Commits: {}", container.commits().count());
    println!("Total Commits Size: {}", display_size(total_size));
    println!(
        "Largest Commit Object Size: {}",
        display_size(largest_commit_size as u64)
    );
    println!(
        "Largest Commit Object Id: {}",
        container
            .commits()
            .lookup_hash_for_index(&largest_commmit_index)
            .unwrap()
    );
    println!(
        "Largest Contributing Commit Size: {}",
        display_size(largest_contributing_size)
    );
    println!(
        "Largest Contributing Commit Object Id: {}\n\n",
        container
            .commits()
            .lookup_hash_for_index(&largest_contributing_commit_index)
            .unwrap()
    );
    println!("Commit report created in: {:?}", start.elapsed());
}

pub fn calc_commit_size(commit: &RwLockReadGuard<'_, Commit>, container: &ObjectContainer) -> u64 {
    let mut total_blob_size: u64 = 0;
    for blob_index in commit.blob_deps() {
        let blob = container.blobs().get_by_index(blob_index).read().unwrap();
        total_blob_size += blob.size_disk() as u64;
    }

    let mut total_tree_size: u64 = 0;
    for tree_index in commit.tree_deps() {
        let tree = container.trees().get_by_index(tree_index).read().unwrap();
        total_tree_size += tree.size_disk() as u64;
    }

    let mut total_tag_size: u64 = 0;
    for tag_index in commit.tag_deps() {
        let tag = container.tags().get_by_index(tag_index).read().unwrap();
        total_tag_size += tag.size_disk() as u64;
    }

    total_blob_size + total_tree_size + total_tag_size
}
