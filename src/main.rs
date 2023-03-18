extern crate core;

use clap::{Parser, Subcommand};
use clipboard::{ClipboardContext, ClipboardProvider};
use prettytable::{color, Attr, Cell, Row, Table};
use std::env;
use std::fs::OpenOptions;
use std::io::Error;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::string::String;

#[derive(Parser)]
#[clap(author, version, about, long_about = None)]
struct Cli {
    /// Sets a custom config file path
    #[clap(short, long, value_parser, value_name = "FILE")]
    config: Option<PathBuf>,

    /// Subcommand to use
    #[clap(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Shows the current configuration
    Show,
    /// launches the ssh command for the selected index of the table or the specified connection name in the table
    Use {
        /// Index of the selected connection
        #[clap(value_parser, value_name = "Selection")]
        selection: String,

        /// Additional args to use in the command
        #[clap(short, long, value_parser, value_name = "args")]
        args: Option<Vec<String>>,

        /// Command to use
        #[clap(short, long, value_parser, value_name = "command", default_value_t = String::from("ssh"))]
        command: String,
    },
    /// exports the ssh command for the selected index of the table or the specified connection name in the table to the clipboard
    Export {
        /// Index of the selected connection
        #[clap(value_parser, value_name = "Selection")]
        selection: String,

        /// Additional args to use in the command
        #[clap(short, long, value_parser, value_name = "args")]
        args: Option<Vec<String>>,

        /// Command to use
        #[clap(short, long, value_parser, value_name = "command", default_value_t = String::from("ssh"))]
        command: String,
    },
    /// launches the scp command for the selected index of the table or the specified connection mane in the table, use "con:"<path> to be replaced with the connection to the selected ssh server
    Copy {
        /// Index of the selected connection
        #[clap(value_parser, value_name = "Selection")]
        selection: String,

        /// Path to copy the file from, use "con:" at the beginning to indicate that this path is on the remote server.
        #[clap(value_parser, value_name = "From")]
        from: String,

        /// Path to copy the file to, use "con:" at the beginning to indicate that this path is on the remote server.
        #[clap(value_parser, value_name = "To")]
        to: String,

        /// Command to use
        #[clap(short, long, value_parser, value_name = "command", default_value_t = String::from("scp"))]
        command: String,
    },
}

fn main() -> Result<(), Error> {
    let cli = Cli::parse();

    let mut buf = env::home_dir().unwrap();
    buf.push(Path::new(".ssh/config"));
    let config_file = match cli.config {
        Some(path) => path,
        None => buf,
    };

    let config_file = config_file.as_path();
    let config_file = config_file.canonicalize().unwrap();
    let config_file = config_file.as_path();

    if cli.command.is_some() {
        let data = read_ssh_config_file(config_file);
        return match &cli.command {
            Some(Commands::Show) => {
                if !(config_file.exists() && config_file.is_file()) {
                    panic!("couldnt open {:#?}", config_file.as_os_str())
                }
                data.map(|data| {
                    let mut table = Table::new();
                    let mut header = true;
                    for row in data {
                        table.add_row(Row::new(
                            row.iter()
                                .map(|cell| {
                                    let table_cell = Cell::new(cell.as_str());
                                    if header {
                                        table_cell
                                            .with_style(Attr::Bold)
                                            .with_style(Attr::ForegroundColor(color::GREEN))
                                    } else {
                                        table_cell.with_style(Attr::ForegroundColor(color::CYAN))
                                    }
                                })
                                .collect::<Vec<Cell>>(),
                        ));
                        if header {
                            header = false;
                        }
                    }
                    // let table = Table::from(data.iter());
                    table.printstd();
                })
            }
            Some(Commands::Use {
                selection,
                args,
                command,
            }) => data.map(|data| {
                let connection_name = get_connection_name(data, selection);
                let mut command = Command::new(command);
                let command = command
                    .arg(connection_name)
                    .stdin(Stdio::inherit())
                    .stdout(Stdio::inherit())
                    .stderr(Stdio::inherit());
                match args {
                    None => {}
                    Some(args) => {
                        command.args(args.iter());
                    }
                };
                command.spawn().unwrap().wait().unwrap();
            }),
            Some(Commands::Export {
                selection,
                args,
                command,
            }) => data.map(|data| {
                let connection_name = get_connection_name(data, selection);
                let mut clipboard: ClipboardContext = ClipboardProvider::new().unwrap();
                let args_str = match args {
                    Some(args) => args.join(" "),
                    None => String::new(),
                };
                clipboard
                    .set_contents(format!("{} {} {}", command, connection_name, args_str))
                    .unwrap();
            }),
            Some(Commands::Copy {
                selection,
                from,
                to,
                command,
            }) => data.map(|data| {
                let connection_name = get_connection_name(data, selection);
                let mut command = Command::new(command);
                let from = from.replace("con:", format!("{}:", connection_name).as_str());
                let to = to.replace("con:", format!("{}:", connection_name).as_str());
                let command = command
                    .arg(from)
                    .arg(to)
                    .stdin(Stdio::inherit())
                    .stdout(Stdio::inherit())
                    .stderr(Stdio::inherit());
                command.spawn().unwrap().wait().unwrap();
            }),
            None => {
                return Ok(());
            }
        };
    }
    Ok(())
}

fn get_connection_name(data: Vec<Vec<String>>, index: &String) -> String {
    match index.trim().parse::<usize>() {
        Ok(index) => {
            if index > data.len() {
                panic!(
                    "incorrect index ({}), max index = {}",
                    index,
                    data.len() - 1
                )
            }
            String::from(&data[index + 1][1])
        }
        Err(_) => {
            if data.iter().filter(|&row| row[1].eq(index)).count() == 0 {
                panic!("no connection in the list with the name {}", index);
            }
            String::from(index)
        }
    }
}

fn read_ssh_config_file(path: &Path) -> Result<Vec<Vec<String>>, Error> {
    OpenOptions::new()
        .read(true)
        .open(path)
        .map(|mut file| {
            let mut data = String::new();
            file.read_to_string(&mut data).expect("Error reading file");
            data
        })
        .map(|data| {
            let mut row = 0;
            let mut first = true;
            let mut data_matrix = vec![];
            for line in data.lines() {
                let line = String::from(line);
                if line.trim().starts_with("Host ") {
                    if first {
                        first = false;
                    } else {
                        row += 1;
                    }
                    let mut data_row = vec![String::new(); 4];
                    data_row[0] = format!("{}", row);
                    data_row[1] = line.trim().replace("Host ", "");
                    data_matrix.push(data_row);
                } else if line.trim().starts_with("HostName ") {
                    let data_row = data_matrix.get_mut(row).unwrap();
                    data_row[2] = line.trim().replace("HostName ", "");
                } else if line.trim().starts_with("user ") {
                    let data_row = data_matrix.get_mut(row).unwrap();
                    data_row[3] = line.trim().replace("user ", "");
                }
            }
            data_matrix
        })
        .map(|data_matrix| {
            let mut data_with_title = vec![vec![
                String::from("Index"),
                String::from("HostName"),
                String::from("Host"),
                String::from("User"),
            ]];
            data_matrix.iter().for_each(|row| {
                let row = row.to_vec();
                data_with_title.push(row)
            });
            data_with_title
        })
}