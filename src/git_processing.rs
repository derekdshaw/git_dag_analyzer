use crate::blob::Blob;
use crate::commit::Commit;
use crate::git_commands::{get_commit_deps, get_tag_deps, list_objects};
use crate::object_collection::{ObjectContainer, Properties};
use crate::tag::Tag;
use crate::tree::Tree;
use anyhow::Result;
use rayon::prelude::*;
use std::sync::{
    atomic::{AtomicU32, AtomicU64, Ordering},
    Arc, RwLock,
};
use std::{
    collections::HashMap,
    fs::File,
    io::{BufRead, BufReader, ErrorKind, Write},
    mem,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};
use tokio::sync::{mpsc, Mutex, Semaphore};
//use rayon::ThreadPoolBuilder;
use tokio::task::JoinSet;

pub fn process_initial_repo(repo_path: &Path, container: &mut ObjectContainer) {
    // Get the list of all objects, their type and sizes from git. Then
    // build up the initial set of in memory objects.
    match list_objects(repo_path) {
        Ok(result) => process_objects(&result, container),
        Err(e) => eprintln!("Error: {e}"),
    }

    println!("Added {} Commits.", container.commits().count());
    println!("Added {} Trees.", container.trees().count());
    println!("Added {} Blobs.", container.blobs().count());
    println!("Added {} Tags.", container.tags().count());
}

// Given a list of objects their sizes and types in a single string with newlines for
// each object. Build up the initial set of containers for each object type.
pub fn process_objects(objects: &str, container: &mut ObjectContainer) {
    println!("Processing objects...");
    let object_lines = objects.lines();

    //for line in object_lines {
    for line in object_lines {
        let object = line.replace('\'', "");
        let properties: Vec<&str> = object.split(" ").collect();

        // There may be a newline at the end of the data, so skip processing that line
        if properties.len() == 4 {
            let hash = properties[1];
            let size = properties[2].parse::<u32>().unwrap();
            let size_disk = properties[3].parse::<u32>().unwrap();

            match properties[0] {
                "commit" => {
                    let index = container.commits().count();
                    container
                        .mut_commits()
                        .add(hash, Commit::new(index, size, size_disk));
                }
                "blob" => {
                    let index = container.blobs().count();
                    container
                        .mut_blobs()
                        .add(hash, Blob::new(index, size, size_disk));
                }
                "tree" => {
                    let index = container.trees().count();
                    container
                        .mut_trees()
                        .add(hash, Tree::new(index, size, size_disk));
                }
                "tag" => {
                    let index = container.tags().count();
                    container
                        .mut_tags()
                        .add(hash, Tag::new(index, size, size_disk));
                }
                _ => println!("Unknown: {}", properties[0]),
            }
        }
    }

    println!("Done processing.");
}

/// This is a wrapper function that will walk all the commits and build a list of just their hashes. This
/// is done to allow for a faster set of processing and alleviate any issues with borrowing during
/// processing.
pub async fn process_all_commit_deps(
    repo_path: &Path,
    container: &ObjectContainer,
    save_load_deps: &Option<PathBuf>,
) -> Result<()> {
    // Build the list of commits to process.
    let mut commits: Vec<String> = Vec::new();
    {
        container
            .commits()
            .object_hash_iter()
            .for_each(|(hash, _index): (&String, &usize)| {
                commits.push(hash.clone());
            });
    }

    let commit_deps: HashMap<String, String>;

    // If we have been asked to save/load the processed deps to save on
    // building those out which can be time consuming.
    if save_load_deps.is_some() {
        // check for an already existing deps file, if we have one load from there.
        // otherwise we save at the end of processing.
        let save_load_path = save_load_deps.as_ref().unwrap();

        // If we already have a saved file, just load the deps into memory for processing.
        if save_load_path.exists() {
            commit_deps = load_deps(save_load_path)?;
        } else {
            // Otherwise we need to build the deps first then save them out to file.
            commit_deps = build_deps_tokio(repo_path, &commits).await;
            save_deps(&commit_deps, save_load_path)?;
        }
    } else {
        // No load/save action requested, just build the deps.
        commit_deps = build_deps_tokio(repo_path, &commits).await;
    }

    process_commit_deps(&commit_deps, container);

    Ok(())
}

/// Build a HashMap of commit hash to dependencies. Where dependencies is a string representing
/// all objects tied to that single commit.
/// Note on processing times. This can take quite a while on a large repo anywhere from 10 min to an hour.
/// Debug and progress information is printed to the console to give an idea of progress.
async fn build_deps_tokio(repo_path: &Path, commits: &[String]) -> HashMap<String, String> {
    let start = Instant::now();
    println!(
        "Getting commit deps. Runs a git command for every commit (This could take a while)..."
    );

    // Just use have the cpu count to keep contention down. Could be a param on the CLI or read from a .env
    let num_cpus = num_cpus::get() / 2;
    let semaphore = Arc::new(Semaphore::new(num_cpus)); // limit the number of concurrent tasks

    let mut set = JoinSet::new();

    // Create progress counter and timing stats
    let progress = Arc::new(AtomicU32::new(0));
    let total_commits = commits.len() as u32;
    let last_reported = Arc::new(AtomicU32::new(0));
    let total_time = Arc::new(AtomicU64::new(0));
    let completed_count = Arc::new(AtomicU64::new(0));

    // Add these for tracking time per percentage
    let last_percent_time = Arc::new(Mutex::new(Instant::now()));
    let last_percent = Arc::new(AtomicU32::new(0));

    // Create a channel for progress updates
    let (tx, mut rx) = mpsc::channel(32);

    // Spawn a task to handle progress reporting
    let progress_task = tokio::spawn({
        let last_reported = last_reported.clone();
        let total_time = total_time.clone();
        let completed_count = completed_count.clone();
        let last_percent_time = last_percent_time.clone();
        let last_percent = last_percent.clone();

        async move {
            while let Some(completed) = rx.recv().await {
                let progress_percent = (completed * 100) / total_commits;
                let last_reported_percent = last_reported.load(Ordering::Relaxed);

                if progress_percent > last_reported_percent {
                    let avg_time_ns = total_time.load(Ordering::Relaxed)
                        / completed_count.load(Ordering::Relaxed).max(1);
                    let avg_time = Duration::from_nanos(avg_time_ns);

                    // Calculate time since last percentage
                    let current_percent = progress_percent;
                    let last_pct = last_percent.load(Ordering::Relaxed);
                    let percent_time = if current_percent > last_pct {
                        let now = Instant::now();
                        let last_time = last_percent_time.lock().await;
                        let elapsed = now.duration_since(*last_time);
                        drop(last_time); // Explicitly drop the guard before re-locking
                        *last_percent_time.lock().await = now;
                        last_percent.store(current_percent, Ordering::Relaxed);
                        elapsed
                    } else {
                        Duration::from_secs(0)
                    };

                    last_reported.store(progress_percent, Ordering::Relaxed);
                    println!(
                        "Progress: {progress_percent}% ({completed} of {total_commits}), Avg: {avg_time:.2?}/task, in {percent_time:.2?}", 
                    );
                }
            }
        }
    });

    // Create a vector of tasks that each return their own HashMap
    for commit_hash in commits.iter() {
        let permit = semaphore.clone().acquire_owned().await.unwrap();
        let progress = progress.clone();
        let tx = tx.clone();
        let total_time = total_time.clone();
        let completed_count = completed_count.clone();

        let commit_hash = commit_hash.to_string();
        let repo_path = repo_path.to_path_buf();

        set.spawn_blocking(move || {
            let start = Instant::now();

            let deps = match get_commit_deps(&repo_path, &commit_hash) {
                Ok(value) => value,
                Err(_) => "".to_string(),
            };

            let mut commit_deps = HashMap::new();

            if let Some(index) = deps.find('\n') {
                let updated_deps = &deps[index + 1..];
                commit_deps.insert(commit_hash, updated_deps.to_string());
            }

            // Update progress and timing
            let task_time = start.elapsed();
            let completed = progress.fetch_add(1, Ordering::Relaxed) + 1;
            total_time.fetch_add(task_time.as_nanos() as u64, Ordering::Relaxed);
            completed_count.fetch_add(1, Ordering::Relaxed);

            let _ = tx.blocking_send(completed);
            drop(permit);
            commit_deps
        });
    }
    // Drop the original sender so the receiver can finish
    drop(tx);

    // Wait for the progress task to finish
    let _ = progress_task.await;

    // Collect results
    let mut final_deps = HashMap::new();

    // For showing a progress indicator
    println!("Merging results of {} tasks...", set.len());
    while let Some(result) = set.join_next().await {
        if let Ok(hashmap) = result {
            final_deps.extend(hashmap);
        }
    }

    println!("\rDone getting deps in {:?}", start.elapsed());
    final_deps
}

/// If specified we will load the data to a file for later processing. The point of this is to
/// save on processing time if are running the commands more than once. Mainly for debugging
/// purposes.
fn load_deps(load_path: &PathBuf) -> Result<HashMap<String, String>> {
    println!("Loading commit deps from file: {load_path:?}");
    let start = Instant::now();
    let mut deps: HashMap<String, String> = HashMap::new();
    let file = File::open(load_path)?;
    let reader = BufReader::new(file);

    let mut have_hash = false;
    let mut hash: String = "".to_string();
    let mut dep_lines = String::new();

    // walk the lines from the file. Once we have a semi colon the next line is a
    // hash. After the has each line is a dep until we see another semi colon, and
    // the process starts over.
    for line_result in reader.lines() {
        match line_result {
            Ok(line) => {
                if line.eq(";") {
                    // look for a semi colon if we find one the next line is the hash
                    if !dep_lines.is_empty() {
                        deps.insert(hash.to_string(), mem::take(&mut dep_lines));
                        dep_lines.clear();
                    }
                    have_hash = true;
                } else if have_hash {
                    hash = line;
                    have_hash = false;
                } else {
                    if line.ends_with(" ") {
                        dep_lines += &line[..line.len() - 1];
                    } else {
                        dep_lines += &line;
                    }

                    dep_lines += "\n";
                }
            }
            Err(e) => match e.kind() {
                ErrorKind::UnexpectedEof => {
                    break;
                }
                _ => return Err(e.into()),
            },
        }
    }

    println!("\rDone loading deps in {:?}", start.elapsed());
    Ok(deps)
}

/// If specified we will save the data to a file for later consumption. The point of this is to
/// save on processing time if are running the commands more than once. Mainly for debugging
/// purposes.
fn save_deps(commit_deps: &HashMap<String, String>, save_path: &PathBuf) -> Result<()> {
    // open the file for writing.
    let mut file = File::create(save_path)?;
    // write a semi colon as a commit delimiter.
    file.write_all(b";\n")?;
    commit_deps
        .iter()
        .try_for_each(|(commit_hash, deps)| -> Result<()> {
            // write the commit hash
            file.write_all(format!("{commit_hash}\n").as_bytes())?;

            // write the deps ( already have \n )
            file.write_all(deps.as_bytes())?;

            if !deps.ends_with("\n") {
                file.write_all(b"\n")?;
            }

            // write a semi colon for next hash.
            file.write_all(b";\n")?;

            Ok(())
        })
}

/// Given a set of existing commits and their depended set. Walk them and build the
/// connections between objects.
pub fn process_commit_deps(commit_deps: &HashMap<String, String>, container: &ObjectContainer) {
    println!("Processing commit deps...");
    let start = Instant::now();

    // Walk all the collected dep strings in parallel
    commit_deps.par_iter().for_each(|(commit_hash, deps)| {
        if let Some(commit_res) = container.commits().get(commit_hash) {
            let mut commit = commit_res.write().unwrap();

            let dep_lines = deps.lines();
            for line in dep_lines {
                if line.len() < 40 {
                    // There may have been a newline at the end of the dep_lines. This
                    // causes there to be an empty item in the list ( line ). Just
                    // ignore it.
                    continue;
                }

                let hash = &line[..40].to_string();
                let mut path = ""; // we can have a tree with no path ( root tree )
                if line.len() > 41 {
                    path = &line[41..];
                }

                // try to get this as a tree, failing that its a blob. If its a commit we can
                // skip it as the data is in the deps.
                let op_tree_index = container.trees().get_index(hash);
                match op_tree_index {
                    Some(tree_index) => {
                        if let Some(tree) = container.trees().get(hash) {
                            let mut tree_guard = tree.write().unwrap();
                            tree_guard.add_path(path);
                            tree_guard.add_commit(commit.hash_index());
                            commit.add_tree_dep(tree_index);
                        } else {
                            println!("Unable to find tree: {hash}");
                        }
                    }
                    None => {
                        let op_blob_index = container.blobs().get_index(hash);
                        match op_blob_index {
                            Some(blob_index) => {
                                if let Some(blob) = container.blobs().get(hash) {
                                    let mut blob_guard = blob.write().unwrap();
                                    blob_guard.add_path(path);
                                    blob_guard.add_commit(commit.hash_index());
                                    commit.add_blob_dep(blob_index);
                                } else {
                                    println!("Unable to find blob: {hash}");
                                }
                            }
                            None => {
                                // this is a commit object and we can skip it.
                            }
                        }
                    }
                }
            }
        }
    });

    println!("processed all commit deps in: {:?}", start.elapsed())
}

pub fn process_tags(repo_path: &Path, container: &ObjectContainer) {
    println!("Processing tags...");
    let start = Instant::now();

    let tag_deps = match get_tag_deps(repo_path) {
        Ok(result) => result,
        Err(e) => {
            println!("Unable to get tag deps. Error: {e}");
            return;
        }
    };

    let lines = tag_deps.lines();
    println!("Processing tag items...");
    let mut previous_tag: Option<&RwLock<Tag>> = None;
    for line in lines {
        let deps: Vec<&str> = line.split(" ").collect();

        let hash = deps[0];
        let label = deps[1];

        // first see if this is a commit
        let commit_index = container.commits().get_index(hash);
        if commit_index.is_some() {
            // this is a commit
            let mut commit = container
                .commits()
                .get_by_index(commit_index.unwrap())
                .write()
                .unwrap();

            // if the previous item was a tag this should be the commit tied to that tag.
            if previous_tag.is_some() {
                let mut tag = previous_tag.unwrap().write().unwrap();
                // verify that the label has the right postfix.
                if line.ends_with("^{}") {
                    // if this line does not end like this then the previous tag is not followed
                    // by a related commit.
                    commit.add_tag_dep(tag.hash_index());
                    tag.add_commit_dep(commit.hash_index());
                } else {
                    let hash = container.tags().lookup_hash_for_index(tag.hash_index());
                    match hash {
                        Some(h) => println!("Tag found with no related commit: {h}"),
                        None => println!("Tag found with no related commit, tag hash not found"),
                    };
                }
                previous_tag = None;
            } else {
                // There is no Tag object tied to this commit.
                // Just a lightweight tag/label
                commit.add_lightweight_tag(label);
            }
        } else {
            // this is a tag
            let tag_opt = container.tags().get(hash);
            if let Some(tag) = tag_opt {
                // this is a tag object
                let mut tag_guard = tag.write().unwrap();
                tag_guard.add_name(label);
            } else {
                println!("Unable to find tag: {hash}");
            }
        }
    }

    println!("Done processing tags in: {:?}", start.elapsed());
}
