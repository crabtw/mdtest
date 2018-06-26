# mdtest

mdtest is a tool for testing [Markdown](http://commonmark.org/) code blocks.

## Usage

Specify the markdown file

```
$ mdtest doc.md
```

mdtest first changes directory to the file belongs in and then tries to run code blocks according to info string.
For unrecognized info string, it just omits the block.

If `--testdir DIR` is specified, it copies all siblings of markdown file into `DIR` and changes directory to it.

```
$ mdtest --testdir _test doc.md
```

## Supported info strings

### `sh` shell command

The `sh` info string indicates that block contents are interpreted as shell commands.
If any command fails, i.e. return code is non-zero, it exits immediately and output stdout and stderr of command.

### `file-exist` check file existence

The `file-exist` info string indicates that each line of block contents is interpreted as file name.
If file does not exist in current directory, it stops checking and print messages.

### `ignore` omit the block

The `ignore` info string indicates that the block is omitted.
It must follow the above info strings and be separated by comma.
