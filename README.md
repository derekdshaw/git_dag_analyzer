# Git Connections

This project is still a work in progress. Its intent is to connect all objects in git, Commits, Trees, Blobs and Tags with each other. The purpose of which is to help discover which objects are contributing most to DAG bloat in Git. This work is the result of my digging into a large mono repo as part of my daily work. That repo DAG was on the order of 60 GB. Which for comparison is is about 12 times larger then the linux kernel repo. 

Currently the system will build a graph connecting all these objects as best as possible. Though its possible that some connections are missed the majority are present. The CLI will output a report of the top contributing objects along with their Hash in order for users to dig deeper directly in Git if needed.

## Current CLI Commands

## Design Decisions

## Performance

## Output Example

## Future Feature Ideas

- Create a web backend with the in memory models as a backing store, and a front end UI to allow for more visual reports accros the entire repo.
- Explore a move to libgit2 as a possible perf gain. Removing the need to call expensive git api's directly. 
- Open to anyone who would like to contribute, or has any feature suggestions, or any corrections/improvements.

