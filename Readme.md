# Multithreaded grep tool

The `mgrep` searches for a specific term within files in a given directory and displays the matching lines along with their corresponding file paths and line numbers.

## Usage

To use the `mgrep` program, follow these steps:

1. Install Rust and Cargo on your system.
2. Clone the repository: `git clone https://github.com/sabry-awad97/mgrep.git`
3. Navigate to the project directory: `cd mgrep`
4. Build the project: `cargo build --release`
5. Run the program with the desired search term and directory:

```shell
./target/release/mgrep "search_term" /path/to/search_directory
```

Replace `"search_term"` with the term you want to search for and `/path/to/search_directory` with the directory where you want to perform the search.

## Dependencies

---

The tool relies on the following external dependencies:

- `structopt`: Used for command-line argument parsing.
- `std::fs`: Provides file system operations.
- `std::path`: Provides path manipulation utilities.
- `std::sync`: Provides synchronization primitives.
- `std::thread`: Enables multi-threading capabilities.

These dependencies are managed through the `Cargo.toml` file.

## Code Overview

The `mgrep` program consists of the following main components:

### SearchError Enum

The `SearchError` enum represents different error cases that can occur during the search process. It provides a way to handle and display specific error messages. The error cases include:

- `IoError`: Represents an I/O error that occurred during file operations.
- `InvalidSearchDir`: Indicates an invalid search directory was provided.

The `From` trait is implemented to convert `std::io::Error` into `SearchError`. Additionally, the `std::fmt::Display` trait is implemented to format and display the error messages.

### Job Struct

The `Job` struct represents a file search job, containing the path to a file that needs to be searched.

### Worklist Struct

The `Worklist` struct manages a list of jobs to be processed. It provides methods to add jobs to the list, retrieve the next job, and finalize the list by adding sentinel jobs.

### SearchResult Struct

The `SearchResult` struct represents a match found within a file. It contains the path, line number, and the matching line itself.

### Worker Struct

The `Worker` struct is responsible for processing search jobs. It takes a search term, a shared worklist, and a shared results vector as input. The `Worker` implements methods to find matches within a file and process the jobs assigned to it. When finding matches, it creates `SearchResult` instances and adds them to the results vector.

### discover_dirs Function

The `discover_dirs` function is a recursive utility function that scans a directory and its subdirectories for files. For each file encountered, it adds a corresponding job to the worklist.

### Cli Struct

The `Cli` struct represents the command-line arguments accepted by the tool. It uses the `structopt::StructOpt` derive macro for argument parsing. The required arguments are:

- `search_term`: The term to search for within the files.
- `search_dir`: The directory to search in.

### main Function

The `main` function is the entry point of the tool. It parses the command-line arguments using `StructOpt`, creates shared instances of the worklist and results vector, spawns a thread to discover directories, and spawns multiple worker threads to process the jobs. Finally, it prints the search results.

## Searching Algorithm

1. The program starts by parsing the command-line arguments to obtain the search term and directory.
2. A worklist is created, which holds the jobs (files to be processed) for the worker threads.
3. Worker threads are spawned based on the specified number.
4. A thread is started to discover directories recursively and add files to the worklist.
5. Each worker thread takes a job from the worklist and searches for the search term in the file.
6. Matching lines are collected in the results vector, protected by a mutex.
7. Once all worker threads finish processing, the results are printed to the console.

## Parallel Processing

The program utilizes multi-threading to parallelize the search process. The number of worker threads can be adjusted as needed.

## Example Output

The program will display the search results in the following format:

```toml
</path/to/file>[line_number]: <matched_line>
</path/to/another/file>[line_number]: <matched_line>
```

Each line contains the file path, line number, and the actual line with the search term.
