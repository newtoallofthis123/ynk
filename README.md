# Y(a)NK

Y(a)nk is a simple command line tool that implements a very basic feature in the terminal that I always wanted to have. It allows you to essentially copy and paste files from one directory to another without having to type out the full path. It's a very simple tool that I made for myself, but I thought it might be useful for others as well.

So essentially, you can go into a directory, like you would do with a GUI, and copy a file or directory. Then you can go to another directory and paste it. It's that simple.

No `cp` or `mv` needed. Just `yank` and `paste`.

## Installation

### Unix

If you are on a unix platform like Linux or Macos, ynk has first class and tested support for it. The best way to install it is to use cargo. If you don't have cargo installed, you can install it from [here](https://rustup.rs/).

Once you have cargo installed, you can install ynk by running the following command:

```bash
cargo install ynk
```

Or, you can also run ~~the install.sh or~~ the `install_linux.py` script in the root of the repository. This will install ynk in `/usr/local/bin` and will also create a `yank` alias for `ynk`.

```bash
./install.sh
```

### Windows

The best way to install ynk on Windows is to use cargo.

```bash
cargo install ynk
```

Moreover, I cannot test it on Windows, so if you find any bugs, please open an issue.

## Usage

### Copying

To copy a file or directory, you can use the `add` command. It takes a single argument which is the path to the file(s) or directory(s) that you want to copy.

```bash
yank add README.md
```

For this you need not provide any arguments. This is because this inherently doesn't do any sort of IO operations. It just stores the path of the file or directory in a database. So it's very fast.

It checks the integrity of the path, converts it into a suitable format, and then stores it in the database. If the path is invalid, it will throw an error.

### Pasting

Pasting is also quite simple. Just use the `paste` command. Here is where you get to essentially tweak what you want to copy over. You can do this with the following options

> **WARNING**: Paste will paste all the files and directories in the current directory. So make sure that you are in the correct directory before pasting.
More customization options will be added in the future.

Before passing in the options, know that the `paste` command is highly optimized for IO tasks. All the way from reading the directory structure (walking the directory tree) to reading and writing files. It uses a lot of threads to do this. So it's very fast.

Moreover, it also by default respects your `.gitignore` file and doesn't copy over an hidden files or directories. To learn how to tweak this, read the options below.

- `-n` or `--no-ignore`: This will make it ignore the `.gitignore` file and copy over all files and directories.
- `--hidden`: This will make it copy over hidden files and directories as well.
- `-s` or `--strict`: This will make sure that any and all IO errors are reported. By default, it will ignore any IO errors and continue with the operation.
- `-f` or `--force`: **TODO** For now, this does nothing. But in the future, this will make it overwrite any files or directories that already exist.
- `--dry-run`: This will make it not actually copy over any files or directories. It will just print out what it would have done.
- `-t` or `--target`: This will make it paste the files and directories in the specified directory. By default, it will paste it in the current directory.
- `-d` or `--delete`: This will make it delete the files and directories from stored in the database after pasting them. Not from the disk.

### Popping

You can pop the last file or directory that you copied. This will remove it from the database and paste all the files and directories in the current directory.

```bash
yank pop
```

### Listing

You can list all the files and directories indexed by ynk in the database.

```bash
yank list
```

### Deleting

You can delete a file or directory from the database.

```bash
yank delete README.md
```

## So it's just a glorified `cp`?

Well, yes and no. For now, it only has a database that sort of abstracts a name and full path. So you can do something like this:

```bash
yank add README.md
```

This would store something like this in the database:

| id | name     | path       | created_at |
|----|----------|------------|------------|
| 1  | README.md| /home/user/README.md | 2020-01-01 00:00:00 |

So, when you paste, it would basically just read the entire file, store it in temporary memory, and then write it to the current directory. So it's not really a `cp` or `mv` because it doesn't actually move the file. It just reads and writes it.

## What about the pasting?

Pasting is at the core of this tool. The indexed files and directories are stored in a database. So when you paste, it will read the file's exact path from the database, and then read the file, and then write it to the current directory.
If the path it is reading from is invalid, it will throw an error.

The IO operation itself is powered by multiple threads, almost like a thread pool. So it's very fast. Every file IO is done in a separate thread. So it's very fast.
The Reading and writing of the files is done in chunks powered by `tokio`'s fs module, maintaining safety and speed.

The file tree is walked using `walkdir`, while making sure that it respects the `.gitignore` file. All of this is done parallely in a thread pool with a in memory static cache.

## What DB do I need to install?

You need not install any database.

The database is a simple SQLite3 database, the drivers for which are embedded in the binary. So you don't need to install anything else. This keeps the binary a single file, and makes it easy to install and use.

The db functionality is all abstracted away in a separate module, so it's easy to swap out the database if needed.

## TODO

- [ ] Add a `--force` option to overwrite files and directories.
- [x] Add a `--target` option to paste files and directories in a specific directory. (release 0.1.2)
- [ ] Add config file support.
- [ ] Add permanent storage option.
- [ ] More explicit error handling.

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Contributing

ynk is an open source project and contributions are welcome. You can contribute in many ways like filing issues, fixing bugs, suggesting features, etc. You can read the contributing guidelines [here](CONTRIBUTING.md).

I am quite new to Rust, so if you find any bugs or have any suggestions, please open an issue. I would love to hear your feedback.

Thanks for trying out ynk!
