use crate::blob::Blob;
use crate::commit::Commit;
use crate::git_commands::{get_commit_deps, list_objects, get_tag_deps};
use crate::object_collection::{ObjectContainer, Properties};
use crate::tag::Tag;
use crate::tree::Tree;
use anyhow::Result;
use std::{ path::{Path,PathBuf},
            collections::HashMap,
            fs::File,
            io::{ Write, BufReader, BufRead, ErrorKind },
            sync::RwLock, 
            time::Instant };
use rayon::prelude::*;
use rayon::ThreadPoolBuilder;

pub fn process_initial_repo(repo_path: &Path, container: &mut ObjectContainer) {

    // Get the list of all objects, their type and sizes from git. Then
    // build up the initial set of in memory objects.
    match list_objects(repo_path) {
        Ok(result) => process_objects(&result, container),
        Err(e) => eprintln!("Error: {}", e),
    }

    println!("Added {} Commits.", container.commits().count());
    println!("Added {} Trees.", container.trees().count());
    println!("Added {} Blobs.", container.blobs().count());
    println!("Added {} Tags.", container.tags().count());
}

// Given a list of objects their sizes and types in a single string with newlines for
// each object. Build up the initial set of containers for each object type.
pub fn process_objects(objects: &str, container: &mut ObjectContainer) {
    println!("Processing objects");
    let object_lines: Vec<&str> = objects.split("\n").collect();

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
                    container.mut_commits().add(hash, Commit::new(index, size, size_disk));
                },
                "blob" => {
                    let index = container.blobs().count();
                    container.mut_blobs().add(hash, Blob::new(index, size, size_disk));
                },
                "tree" => {
                    let index = container.trees().count();
                    container.mut_trees().add(hash, Tree::new(index, size, size_disk));
                },
                "tag" => {
                    let index = container.tags().count();
                    container.mut_tags().add(hash, Tag::new(index, size, size_disk));
                },
                _ => println!("Unknown: {}", properties[0]),
            }
        }
    }

    println!("Done processing.");
}

/// This is a wrapper function that will walk all the commits and build a list of just their hashes. This
/// is done to allow for a faster set of processing and alleviate any issues with borrowing during
/// processing. 
pub fn process_all_commit_deps(
    repo_path: &Path,
    container: &ObjectContainer,
    save_load_deps: &Option<PathBuf>
) -> Result<()> {

    // Build the list of commits to process.
    let mut commits: Vec<String> = Vec::new();
    {
        container.commits().object_hash_iter().for_each(|(hash, _index): (&String, &usize)| {
            commits.push(hash.clone());
        });
    }
    
    let commit_deps: RwLock<HashMap<String, String>>;

    // If we have been asked to save/load the processed deps to save on
    // building those out which can be time consuming. 
    if save_load_deps.is_some() {
        // check for an already existing deps file, if we have one load from there.
        // otherwise we save at the end of processing.
        let save_load_path = save_load_deps.as_ref().unwrap();

        // If we already have a saved file, just load the deps into memory for processing. 
        if save_load_path.exists() {
            commit_deps = load_deps(&save_load_path)?;
        } else {
            // Otherwise we need to build the deps first then save them out to file. 
            commit_deps = build_deps(repo_path, &commits);
            save_deps(&commit_deps, &save_load_path)?;
        }
    } else {
        // No load/save action requested, just build the deps. 
        commit_deps = build_deps(repo_path, &commits);
    }

    process_commit_deps(&commit_deps, container);

    Ok(())
}

/// Build a HashMap of commit hash to dependencies. Where dependencies is a string representing
/// all objects tied to that single commit. 
/// Note on processing times. This can take quite a while on a large repo anywhere from 10 min to an hour.
/// No current indicator of progress has been implemented. So folks just have to wait for it to complete.
/// return the rwlocked hashmap to avoid cloning on the way out.
fn build_deps(repo_path: &Path, commits: &Vec<String>) -> RwLock<HashMap<String, String>> {
    println!("Getting commit deps (This could take a while)...");
    let start = Instant::now();
    let commit_deps_lock:RwLock<HashMap<String, String>> = RwLock::new(HashMap::new());

    // parallel this loop with a low thread count, so as not to overload git with too many requests
    // 8 was chosen as it seems to be the sweet spot for performance. More threads create lock contention
    // either within git, or this code.
    let thread_count = if rayon::current_num_threads() > 8 { 8 } else { rayon::current_num_threads() };
    let thread_pool = ThreadPoolBuilder::new().num_threads(thread_count).build().unwrap();
    thread_pool.install(|| {
        commits.par_iter().for_each(|commit_hash| {

            let deps = match get_commit_deps(repo_path, commit_hash) {
                Ok(value) => value,
                Err(e) => {
                    println!("Unable to get commit deps. Error {}", e);
                    "".to_string()
                }
            };

            let mut commit_deps = commit_deps_lock.write().unwrap();
            
            // remove the extra commit hash
            let op_index = deps.find("\n");
            match op_index {
                Some(index) => {
                    let updated_deps = &deps[index+1..];
                    commit_deps.insert(commit_hash.to_string(), updated_deps.to_string());
                },
                None => {}
            }
            
        });
    });

    println!("\rDone getting deps in {:?}", start.elapsed());

    commit_deps_lock
}

/// If specified we will load the data to a file for later processing. The point of this is to
/// save on processing time if are running the commands more than once. Mainly for debugging
/// purposes.
fn load_deps(load_path: &PathBuf) -> Result<RwLock<HashMap<String, String>>> {

    println!("Loading commit deps from file: {:?}", load_path);
    let start = Instant::now();
    let deps:RwLock<HashMap<String, String>> = RwLock::new(HashMap::new());
    let file = File::open(load_path)?;
    let reader = BufReader::new(file);

    let mut have_hash = false;
    let mut hash:String = "".to_string();
    let mut dep_lines = "".to_string();

    // walk the lines from the file. Once we have a semi colon the next line is a
    // hash. After the has each line is a dep until we see another semi colon, and
    // the process starts over.
    for line_result in reader.lines() {      
        match line_result {
            Ok(line) => {
                if line.eq(";") { // look for a semi colon if we find one the next line is the hash
                    if dep_lines.len() > 0
                    {
                        deps.write().unwrap().insert(hash.clone(), dep_lines.clone());
                        dep_lines.clear();
                    }
                    have_hash = true;
                } else if have_hash {
                    hash = line;
                    have_hash = false;
                } else {
                    if line.ends_with(" ") {
                        dep_lines += &line[..line.len() -1];
                    } else {
                        dep_lines += &line;
                    }
                    
                    dep_lines += "\n";
                }
            },
            Err(e) => match e.kind() {
                ErrorKind::UnexpectedEof => {
                    break;
                },
                _ => return Err(e.into()),
            }
        }
    }

    println!("\rDone loading deps in {:?}", start.elapsed());
    Ok(deps)
}

/// If specified we will save the data to a file for later consumption. The point of this is to
/// save on processing time if are running the commands more than once. Mainly for debugging
/// purposes.
fn save_deps(commit_deps: &RwLock<HashMap<String, String>>, save_path: &PathBuf) -> Result<()> {

    // open the file for writing.
    let mut file = File::create(save_path)?;
    // write a semi colon as a commit delimiter.
    file.write(b";\n")?;
    let commits = commit_deps.read().unwrap();
    commits.iter().try_for_each(|(commit_hash, deps)| -> Result<()> {

        // write the commit hash
        file.write(format!("{}\n", commit_hash).as_bytes())?;

        // write the deps ( already have \n )
        file.write(deps.as_bytes())?;

        if !deps.ends_with("\n") {
            file.write(b"\n")?;
        }

        // write a semi colon for next hash.
        file.write(b";\n")?;
        
        Ok(())
    })
}

/// Given a set of existing commits and their depended set. Walk them and build the 
/// connections between objects.
pub fn process_commit_deps (
    commit_deps: &RwLock<HashMap<String, String>>,
    container: &ObjectContainer,
) {
    println!("Processing commit deps...");
    let start = Instant::now();

    // Walk all the collected dep strings in parallel
    commit_deps.read().unwrap().par_iter().for_each(|(commit_hash, deps)| {
        let mut commit;
        let commit_opt = container.commits().get(commit_hash);
        match commit_opt {
            Some(commit_res) => {
                commit = commit_res.write().unwrap();
            }
            None => {
                println!("Unable to find commit: {}", commit_hash);
            }
        }
        let dep_lines: Vec<&str> = deps.split("\n").collect();
        for (_index, line) in dep_lines.iter().enumerate() {

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
                    let tree_opt = container.trees().get(hash);
                    match tree_opt {
                        Some(tree) => {
                            let mut tree_guard = tree.write().unwrap();
                            tree_guard.add_path(path);
                            tree_guard.add_commit(commit.hash_index());
                            commit.add_tree_dep(tree_index);
                        },
                        None => {
                            println!("Unable to find tree: {}", hash);
                        }
                    }
                },
                None => {
                    let op_blob_index = container.blobs().get_index(hash);
                    match op_blob_index {
                        Some(blob_index) => {
                            let blob_opt = container.blobs().get(hash);
                            match blob_opt {
                                Some(blob) => {
                                    let mut blob_guard = blob.write().unwrap();
                                    blob_guard.add_path(path);
                                    blob_guard.add_commit(commit.hash_index());
                                    commit.add_blob_dep(blob_index);
                                },
                                None => {
                                    println!("Unable to find blob: {}", hash);
                                }
                            }
                        },
                        None => {
                           // this is a commit object and we can skip it. 
                        }
                    }
                }
            }
        };
    });

    println!("processed all commit deps in: {:?}", start.elapsed())
}

pub fn process_tags(repo_path: &Path, container: &ObjectContainer) {
    println!("Processing tags...");
    let start = Instant::now();

    let tag_deps = match get_tag_deps(repo_path) {
        Ok(result) => result,
        Err(e) => {
            println!("Unable to get tag deps. Error: {}", e);
            return
        }
    };

    let lines: Vec<&str> = tag_deps.split("\n").collect();
    println!("Processing {} items.", lines.len());
    let mut previous_tag:Option<&RwLock<Tag>> = None;
    lines.iter().for_each(|line| {
        let deps: Vec<&str> = line.split(" ").collect();

        let hash = deps[0];
        let label = deps[1];

        // first see if this is a commit
        let commit_index = container.commits().get_index(hash);
        if commit_index.is_some() {
            // this is a commit
            let mut commit = container.commits().get_by_index(commit_index.unwrap()).write().unwrap();
            

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
                        Some(h) => println!("Tag found with no related commit: {}", h),
                        None => println!("Tag found with no related commit, tag hash not found")
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
            match tag_opt {
                Some(tag) => {
                    // this is a tag object
                    let mut tag_guard = tag.write().unwrap();
                    tag_guard.add_name(label);
                },
                None => {
                    println!("Unable to find tag: {}", hash);
                }
            }
        }
    });

    println!("Done processing tags in: {:?}", start.elapsed());
}