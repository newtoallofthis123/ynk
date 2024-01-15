# Y(a)NK

Y(a)nk is a simple command line tool that implements a very basic feature in the terminal that I always wanted to have. It allows you to essentially copy and paste files from one directory to another without having to type out the full path. It's a very simple tool that I made for myself, but I thought it might be useful for others as well.

So essentially, you can go into a directory, like you would do with a GUI, and copy a file or directory. Then you can go to another directory and paste it. It's that simple.

No `cp` or `mv` needed. Just `yank` and `paste`.

## So it's just a glorified `cp`?

Well, yes and no. For now, it only has a database that sort of abstracts a name and full path. So you can do something like this:

```bash
yank README.md
paste
```

This would store something like this in the database:

| id | name     | path       | created_at |
|----|----------|------------|------------|
| 1  | README.md| /home/user/README.md | 2020-01-01 00:00:00 |

So, when you paste, it would basically just read the entire file, store it in temporary memory, and then write it to the current directory. So it's not really a `cp` or `mv` because it doesn't actually move the file. It just reads and writes it.

## Under active development

It is still under active development and this GitHub repo is just to make sure that I don't lose the code. I will be adding more features and making it more robust.
