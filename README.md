# Git Connections

This project is still a work in progress. Its intent is to connect all objects in git, Commits, Trees, Blobs and Tags with each other. The purpose of which is to help discover which objects are contributing most to DAG bloat in Git. This work is the result of my digging into a large mono repo as part of my daily work. That repo DAG was on the order of 60 GB. Which for comparison is is about 12 times larger then the linux kernel repo. 

Currently the system will build a graph connecting all these objects as best as possible. Though its possible that some connections are missed the majority are present. The CLI will output a report of the top contributing objects along with their Hash in order for users to dig deeper directly in Git if needed.

## Current CLI Commands

The Git DAG Analyzer provides the following command line interface:

### Basic Usage
```
git-dag-analyzer --repo <REPO_PATH> <COMMAND> [OPTIONS]
```

### Commands

#### `reports`
Generate various reports about the repository.

**Options:**
- `-a, --all`: Generate all available reports (commits, trees, and blobs)
- `-c, --commits`: Generate commit report
- `-t, --trees`: Generate tree report
- `-b, --blobs`: Generate blob report
- `-s, --save-deps <SAVE_LOCATION>`: Save processed commit dependencies to a file for future use

**Examples:**
```
# Generate all reports
git-dag-analyzer --repo /path/to/repo reports --all

# Generate only commit report
git-dag-analyzer --repo /path/to/repo reports --commits

# Save processed data for future use
git-dag-analyzer --repo /path/to/repo reports --all --save-deps deps.json
```

#### `process-only`
Process repository data without generating reports. Useful for preparing data for later analysis.

**Options:**
- `-a, --all`: Process all data (commits and tags)
- `-c, --commits`: Process commit data only
- `-l, --labels`: Process tag data only
- `-s, --save-deps <SAVE_LOCATION>`: Save processed commit dependencies to a file

**Examples:**
```
# Process all data and save for later
git-dag-analyzer --repo /path/to/repo process-only --all --save-deps deps.json

# Process only commit data
git-dag-analyzer --repo /path/to/repo process-only --commits
```

### Required Arguments
- `-r, --repo <REPO_PATH>`: Path to the git repository to analyze

### Version Information
Use `--version` to display version information.

## Design Decisions

## Performance

## Output Example

### Reports

This is a report generated from the VSCode repo on GitHub.

```
Building commit report...

Commit Report
-------------------------------------------------------
Total Commits: 134216
Total Commits Size: 53.17 MB
Largest Commit Object Size: 4.67 KB
Largest Commit Object Id: 06df95754da587af444c65bbefbe6aaa7919d25e
Largest Contributing Commit Size: 261.32 MB
Largest Contributing Commit Object Id: 1fe872e131c0b79e006abdc87c78f2ea26bc82f6


Commit report created in: 251.1107ms
Building tree report...

Tree Report
-------------------------------------------------------
Total Trees: 1112284
Total Trees Size: 136.16 MB
Largest Tree Object Size: 4.28 KB
Largest Tree Object Id: 4520f0f17aa59be798716534c4b9ee02571ca083
Most Trees at Path:
Count Most Trees at Path: 132779
Most Trees at Path Total Size: 10.72 MB


Tree report created in: 110.9533ms
Building blob report...

Blob Report
-------------------------------------------------------
Total Blobs: 407643
Total Blobs Size: 742.81 MB
Top 10 Largest Blobs:
        Blob Size: 4.63 MB, Hash: 4053d6e811d4e166d6eaa126c258ec82d86a7980
        Blob Size: 2.32 MB, Hash: 7df50c520fd47cdff978a7a59b773f9ac0dc2303
        Blob Size: 2.20 MB, Hash: c9ed1d359195ce16ac648dfa9cbf89fcbdb6db59
        Blob Size: 1.67 MB, Hash: 894873c6c08d5710dfec882467b3dd4c6e289295
        Blob Size: 1.51 MB, Hash: d3e770bb97cfdcaa8c98eb922c9591d91d3b0b6f
        Blob Size: 1.42 MB, Hash: 23417ed8b8845609e70ef707257af11cfa3a65f8
        Blob Size: 1.42 MB, Hash: faffe54db392cbd7a2f6d097bd204fa7a6b8d973
        Blob Size: 688.10 KB, Hash: ee0a24b51ebfcdc11cf340d4ae264e637238da64
        Blob Size: 385.33 KB, Hash: 6fbcfed92d0f5036b3acde7e8d7c724d20b928b1
        Blob Size: 648 bytes, Hash: 5f6e1cd99f3a7cd4760f0150fb6998ad33aa21d1
Blob report created in: 10.1691ms
```

## Future Feature Ideas

- Create a web backend with the in memory models as a backing store, and a front end UI to allow for more visual reports across the entire repo.
- Explore a move to libgit2 as a possible perf gain. Removing the need to call expensive git api's directly. 
- Open to anyone who would like to contribute, or has any feature suggestions, or any corrections/improvements.
