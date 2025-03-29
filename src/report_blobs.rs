use crate::object_collection::{ObjectContainer, Properties};
use crate::utils::display_size;
use std::time::Instant;

pub fn report_blobs(container: &ObjectContainer) {

    println!("Building blob report...");
    let start = Instant::now();
    let mut total_size:u64 = 0;
    let mut top_ten_size:Vec<(u32, usize)> = Vec::new();

    for rw_blob in container.blobs().object_iter() {
        let blob = rw_blob.read().unwrap();
        total_size += blob.size_disk() as u64;
        
        // Add until we have 10 items
        if top_ten_size.len() < 10 {
            top_ten_size.push((blob.size_disk(), *blob.hash_index()));
            top_ten_size.sort(); // ascending _by(|a, b| a.cmp(b));
        } else {
            // we already have 10 items, does this one fit in the list.
            if blob.size_disk() > top_ten_size[0].0 {
                let mut insert_index:usize = 10;
                // iterate the items until you find which one to insert, pop off the bottom item
                for (index, value) in top_ten_size.iter().enumerate() {
                    if blob.size_disk() < value.0 {
                        insert_index = index - 1;
                        break;
                    } 
                }

                // If insert index is 10, this size is larger than the largest collected.
                if insert_index == 10 {
                    top_ten_size.insert(9, (blob.size_disk(), *blob.hash_index()));
                } else {
                    top_ten_size.insert(insert_index, (blob.size_disk(), *blob.hash_index()));
                }

                // always remove the smallest item ( top )
                assert!(top_ten_size.len() == 11);
                let _ = top_ten_size.remove(0); // dont need the return

            }
        }
    }

    // resort to descending
    top_ten_size.sort_by(|a, b| b.cmp(a));

    println!();
    println!("Blob Report");
    println!("-------------------------------------------------------");
    println!("Total Blobs: {}", container.blobs().count());
    println!("Total Blobs Size: {}",display_size(total_size));
    println!("Top 10 Largest Blobs:");
    for (size, blob_index) in top_ten_size {
        println!("\tBlob Size: {}, Hash: {}", display_size(size as u64), container.blobs().lookup_hash_for_index(&blob_index).unwrap())
    }
    println!("Blob report created in: {:?}", start.elapsed());
}