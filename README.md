# SSH-VIEW

[![License: MIT OR Apache-2.0](https://img.shields.io/crates/l/clippy.svg)]()
![LoC][lo]

[lo]: https://tokei.rs/b1/github/JuanJMarques/ssh-view?category=code

This is a simple utility for parsing, showing and integrate the info from the `$/.ssh/config` file or a custom ssh configuration file.
 

### Install
Download the binaries from the [releases](https://github.com/JuanJMarques/ssh-view/releases/latest) page.

### Build

```
git clone https://github.com/JuanJMarques/ssh-view.git
cd ssh-view
cargo build --release
cp tartget/release/ssh-view <some-dir-in-your-$Path>/ssh-view
```
### Usage

```
ssh-view 0.2.0

USAGE:
    ssh-view.exe [OPTIONS] [SUBCOMMAND]

OPTIONS:
    -c, --config <FILE>    Sets a custom config file path
    -h, --help             Print help information
    -V, --version          Print version information

SUBCOMMANDS:
    add       Adds a new entry to the ssh config file
    copy      launches the scp command for the selected index of the table or the specified
                  connection mane in the table, use "con:"<path> to be replaced with the connection
                  to the selected ssh server
    delete    Deletes an entry from the ssh config file
    export    exports the ssh command for the selected index of the table or the specified
                  connection name in the table to the clipboard
    help      Print this message or the help of the given subcommand(s)
    show      Shows the current configuration
    use       launches the ssh command for the selected index of the table or the specified
                  connection name in the table
```

list ssh servers
```
$ssh-view show
+-------+-----------+--------------+--------+
| Index | HostName  | Host         | User   |
+-------+-----------+--------------+--------+
| 0     | test1     | 192.168.90.1 | centos |
+-------+-----------+--------------+--------+
| 1     | test2     | 192.168.90.2 | ubuntu |
+-------+-----------+--------------+--------+
```

connect to a server
```
$ssh-view use 0
Last login: Sat Jan 1 00:00:00 2022 from 0.0.0.0
[ubuntu@ip-0-0-0-0 ~]$ 
```

export the connection data to the clipboard
```
$ssh-view export 0
```

add entry to ssh config file
```
$ssh-view add localhost 127.0.0.1 root

$ssh-view show
+-------+-----------+--------------+--------+
| Index | HostName  | Host         | User   |
+-------+-----------+--------------+--------+
| 0     | test1     | 192.168.90.1 | centos |
+-------+-----------+--------------+--------+
| 1     | test2     | 192.168.90.2 | ubuntu |
+-------+-----------+--------------+--------+
| 2     | localhost | 127.0.0.1    | root   |
+-------+-----------+--------------+--------+
```

delete entry from ssh config file
```
$ssh-view show
+-------+-----------+--------------+--------+
| Index | HostName  | Host         | User   |
+-------+-----------+--------------+--------+
| 0     | test1     | 192.168.90.1 | centos |
+-------+-----------+--------------+--------+
| 1     | test2     | 192.168.90.2 | ubuntu |
+-------+-----------+--------------+--------+
| 2     | localhost | 127.0.0.1    | root   |
+-------+-----------+--------------+--------+

$ssh-view delete 2
The host "localhost" will be deleted, are you sure?
Type "yes" to confirm
yes

$ssh-view show
+-------+----------+--------------+--------+
| Index | HostName | Host         | User   |
+-------+----------+--------------+--------+
| 0     | test1    | 192.168.90.1 | centos |
+-------+----------+--------------+--------+
| 1     | test2    | 192.168.90.2 | ubuntu |
+-------+----------+--------------+--------+
```