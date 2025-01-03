# Ynk Commands and Flags

**add [--preserve] [FILES]**: Adds multiple files and directories specificed by FILES into the ynk store. Left empty, it defaults to adding the current working directory.

Flags:

- `--preserve`: Specifies that the file stucture that is specified has to be preserved. This means that if you add `../dir`, while pasting ynk pastes  it in `../output`.

*Example*: `ynk add --dir ~/Projects/ns`

**pop [--noignore -n] [--all -a] [--overwrite] [--strict] [-o --output PATH]**: Pops the last entry added to the ynk store. Popping also deletes the entry from the ynk store and pastes it as specified.

Flags:

- `--noignore`: By default ynk respects the .gitignore folder found in directory to paste. This bypasses it and asks ynk to ignore the .gitignore file (ironic).

- `--all`: Ynk also ignores hidden files by default. This flag includes the hidden directories in the popping.

- `--overwrite`: Gives ynk permission to overwrite files while pasting.

- `--strict`: By default ynk doesn't mind if a file errors while pasting, however in strict mode, any and all errors are reported.

- `--output PATH`: Specfies the output path of the popped entry

**paste [--noignore -n] [--all -a] [--overwrite] [--strict] [-o --output PATH] [-r --range 0..n] [QUERIES]**: Pastes entries from the ynk store. Gives options to query only selected entries or paste ranges.

Flags:

- `--noignore`: By default ynk respects the .gitignore folder found in directory to paste. This bypasses it and asks ynk to ignore the .gitignore file (ironic).

- `--all`: Ynk also ignores hidden files by default. This flag includes the hidden directories in the popping.

- `--overwrite`: Gives ynk permission to overwrite files while pasting.

- `--strict`: By default ynk doesn't mind if a file errors while pasting, however in strict mode, any and all errors are reported.

- `--output PATH`: Specfies the output path of the popped entry

- `--range <RANGE>`: Specifies the range of id's of entries to paste. This follows the rust iterator syntax of `start..end` or `start..` or `start`

- `[QUERIES]`: The queries to filter the entries by
  Example: 
  
  - `ynk paste ~/Projects` pastes all the entries whose path starts with ~/Projects. 
  
  - `ynk paste ns` pastes any references to `ns` name in entries.
  
  - `ynk paste -r 1..4` pastes all entries whose id's are [1,2,3,4].
  
  - `ynk paste 1 3` pastes the entry with id 1 and 3 

**list [--size] [--noignore -n] [--all -a]**: List entries in the ynk store.

Flags:

- `--size`: Also calculates and prints the size of each entry on the disk. This can somethings be inaccurate and changes according to the usuage of `--all` and `--noignore`.

- `--noignore`: By default ynk respects the .gitignore folder found in directory to paste. This bypasses it and asks ynk to ignore the .gitignore file (ironic).

- `--all`: Ynk also ignores hidden files by default. This flag includes the hidden directories in the popping.

**delete [QUERIES]**: Deletes entries from the ynk store. This only deletes entries from the ynk store and doesn't modify or delete the actual files or folders.

Flags:

`[QUERIES]`: The queries to filter the entries by
Example:

- `ynk delete ~/Projects` deletes all the entries whose path starts with ~/Projects.

- `ynk delete ns` deletes any references to `ns` name in entries.

**clear**: Clears all entries from ynk store

**completions [SHELL]**: Prints out generation file for the given shell

Supported shells are: Fish, Zsh, Bash and Powershell. Look into [Shell Completions](#Shell-Completions) for more info.

**help [COMMAND]**: Look at this usuage information in the shell.


## Config Options

The config file has the following options:

```bash
strict = false
ignore = true
all = false
overwrite = false
delete = false
prompt = true
show_splash = true
calculate_size = true
preserve_structure = false
```

Each of this means the following:

- `strict`: Enables strict mode for pasting operations
- `ignore`: Ignore files and directories mentioned in the `.gitignore` file
- `all`: Include hidden files while pasting
- `overwrite`: Overwrite files and folders while pasting
- `delete`: Delete the entry after pasting the entry
- `prompt`: Prompt the user before critical operations such as clear etc
- `show_splash`: Decides if ynk should disable it's banner. This is disabled automatically in stdout mode.
- `calculate_size`: Calculates the size while listing entries.
- `preserve_structure`: Preserves the structure of the entry while adding
